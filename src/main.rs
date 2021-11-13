#![feature(derive_default_enum)]

mod data;
mod ops;

use data::*;

#[derive(argh::FromArgs, Debug)]
/// Manage installed DankBSD software images
struct Args {
    #[argh(option, default = "infer_system_pool()")]
    /// the zpool to use for operations on individual images
    pool: String,
    #[argh(subcommand)]
    cmd: Cmd,
}

#[derive(argh::FromArgs, Debug)]
#[argh(subcommand)]
enum Cmd {
    New(NewCmd),
    Commit(CommitCmd),
}

#[derive(argh::FromArgs, Debug)]
/// Create a new WIP image
#[argh(subcommand, name = "new")]
struct NewCmd {
    #[argh(option, default = "data::ImageType::App")]
    /// type of the created image
    kind: ImageType,
    #[argh(option)]
    /// parent image to base the current one on
    from: Option<ImageRef>,
    #[argh(positional)]
    image: ImageRef,
}

#[derive(argh::FromArgs, Debug)]
/// Finalize a WIP image, turning it into a read-only one that can be published
#[argh(subcommand, name = "commit")]
struct CommitCmd {
    #[argh(positional)]
    image: ImageRef,
}

fn main() {
    let args: Args = argh::from_env();
    let zfs = libzetta::zfs::DelegatingZfsEngine::new().unwrap();
    match args.cmd {
        Cmd::New(NewCmd {
            kind,
            from: None,
            image,
        }) => {
            println!("{}", ops::create(&zfs, args.pool, image, kind).unwrap());
        }
        Cmd::New(NewCmd {
            kind: _,
            from: Some(from),
            image,
        }) => {
            println!("{}", ops::clone(&zfs, args.pool, from, image).unwrap());
        }
        Cmd::Commit(CommitCmd { image }) => ops::commit(&zfs, args.pool, image).unwrap(),
    }
}

fn infer_system_pool() -> String {
    use systemstat::Platform;
    match systemstat::System::new().mount_at("/") {
        Ok(mount) if mount.fs_type == "zfs" => {
            mount.fs_mounted_from[..mount.fs_mounted_from.find('/').unwrap()].to_string()
        }
        Ok(_) => panic!("Root filesystem is not ZFS, provide the --pool option manually to choose a ZFS pool to operate on"),
        Err(e) => panic!("Could not retrieve root filesystem info: {:?}", e)
    }
}
