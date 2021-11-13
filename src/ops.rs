use crate::data::*;
use libzetta::zfs::{self, DelegatingZfsEngine as Zfs, ZfsEngine};
use std::{collections::HashMap, process::Command, str::FromStr};

static ROOT_DATASET: &'static str = "dankup-store";
static PROP_WIP: &'static str = "dankbsd.dankup:wip";
static PROP_KIND: &'static str = "dankbsd.dankup:kind";

trait ZfsExt {
    fn ensure_parent(&self, name: String, kind: ImageType) -> anyhow::Result<()>;
}
impl ZfsExt for Zfs {
    fn ensure_parent(&self, name: String, kind: ImageType) -> anyhow::Result<()> {
        if self.exists(&name)? {
            if let zfs::Properties::Filesystem(ps) = self.read_properties(&name)? {
                if let Some(pkind) = ps.unknown_properties().get(PROP_KIND) {
                    let pkind = ImageType::from_str(pkind)?;
                    if pkind != kind {
                        anyhow::bail!(
                            "Parent dataset {} has kind {} instead of {}",
                            name,
                            pkind,
                            kind
                        );
                    }
                }
            } else {
                anyhow::bail!("Parent dataset {} is not a filesystem", name);
            }
        } else {
            let req = zfs::CreateDatasetRequestBuilder::default()
                .name(name)
                .kind(zfs::DatasetKind::Filesystem)
                .checksum(zfs::Checksum::Skein)
                .compression(zfs::Compression::Zstd9)
                .atime(false)
                .exec(true)
                .readonly(false)
                .xattr(true)
                .setuid(kind == ImageType::System)
                .devices(kind == ImageType::System)
                .user_properties(Some(HashMap::from([(
                    PROP_KIND.to_string(),
                    kind.to_string(),
                )])))
                .build()?;
            eprintln!("{:?}", req);
            self.create(req)?;
        }
        Ok(())
    }
}

pub fn create(zfs: &Zfs, pool: String, image: ImageRef, kind: ImageType) -> anyhow::Result<String> {
    let name = format!("{}/{}/{}", pool, ROOT_DATASET, image.as_zpath());
    zfs.ensure_parent(format!("{}/{}/{}", pool, ROOT_DATASET, image.image), kind)?;
    zfs.create(
        zfs::CreateDatasetRequestBuilder::default()
            .name(name.clone())
            .kind(zfs::DatasetKind::Filesystem)
            .readonly(false)
            .user_properties(Some(HashMap::from([(
                PROP_WIP.to_string(),
                "true".to_string(),
            )])))
            .build()?,
    )?;
    Command::new("zfs").arg("mount").arg(&name).status()?;
    Ok(name)
}

pub fn clone(zfs: &Zfs, pool: String, from: ImageRef, to: ImageRef) -> anyhow::Result<String> {
    let from_name = format!("{}/{}/{}@S", pool, ROOT_DATASET, from.as_zpath());
    let to_name = format!("{}/{}/{}", pool, ROOT_DATASET, to.as_zpath());
    if !zfs.exists(&from_name)? {
        anyhow::bail!("Dataset {} does not exist", from_name);
    }
    let kind =
        if let zfs::Properties::Snapshot(ps) = zfs.read_properties(&from_name)? {
            ImageType::from_str(ps.unknown_properties().get(PROP_KIND).ok_or(
                anyhow::format_err!("Dataset {} does not have a kind", from_name),
            )?)?
        } else {
            anyhow::bail!("Dataset {} is not a snapshot", from_name);
        };
    zfs.ensure_parent(format!("{}/{}/{}", pool, ROOT_DATASET, to.image), kind)?;
    zfs.clone_snapshot(
        to_name.clone(),
        from_name,
        Some(HashMap::from([(PROP_WIP.to_string(), "true".to_string())])),
    )?;
    Command::new("zfs").arg("mount").arg(&to_name).status()?;
    Ok(to_name)
}

pub fn commit(zfs: &Zfs, pool: String, image: ImageRef) -> anyhow::Result<()> {
    let name = format!("{}/{}/{}", pool, ROOT_DATASET, image.as_zpath());
    if !zfs.exists(&name)? {
        anyhow::bail!("Dataset {} does not exist", name);
    }
    if let zfs::Properties::Filesystem(ps) = zfs.read_properties(&name)? {
        if !ps.mounted() {
            anyhow::bail!("Dataset {} is not mounted", name);
        }
        if ps.unknown_properties().get(PROP_WIP).map(|x| x as &str) != Some("true") {
            anyhow::bail!("Dataset {} is not a WIP image", name);
        }
        Command::new("zfs")
            .arg("set")
            .arg(format!("{}=false", PROP_WIP))
            .arg("readonly=on")
            .arg(&name)
            .status()?;
        let snap = format!("{}@S", name);
        zfs.snapshot(&[snap.into()], None)?;
        Ok(())
    } else {
        anyhow::bail!("Dataset {} is not a filesystem", name);
    }
}
