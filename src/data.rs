#[derive(Default, Debug, PartialEq)]
pub enum ImageType {
    #[default]
    App,
    Devbox,
    System,
}

impl std::str::FromStr for ImageType {
    type Err = std::fmt::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "app" => Ok(Self::App),
            "devbox" => Ok(Self::Devbox),
            "system" => Ok(Self::System),
            _ => Err(std::fmt::Error),
        }
    }
}

impl std::fmt::Display for ImageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::App => "app",
                Self::Devbox => "devbox",
                Self::System => "system",
            }
        )
    }
}

#[derive(Debug, PartialEq)]
pub struct ImageRef {
    pub image: String,
    pub epoch: u32,
    pub version: String,
}

impl std::fmt::Display for ImageRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}_{}", self.image, self.epoch, self.version,)
    }
}

impl ImageRef {
    pub fn as_zpath(&self) -> String {
        format!("{}/{}_{}", self.image, self.epoch, self.version,)
    }
}

#[derive(pest_derive::Parser)]
#[grammar_inline = r#"
char = { ASCII_ALPHANUMERIC | "." | "_" }
image = { char+ }
epoch = { ASCII_DIGIT+ }
version = { char+ }
iref = { SOI ~ image ~ "@" ~ epoch ~ "_" ~ version ~ EOI }
"#]
struct ImageRefParser;

impl std::str::FromStr for ImageRef {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use pest::Parser;
        let mut image = None;
        let mut epoch = None;
        let mut version = None;
        if let Some(o) = ImageRefParser::parse(Rule::iref, s)?.next() {
            for i in o.into_inner() {
                match i.as_rule() {
                    Rule::image => {
                        image = Some(i.as_str().to_string());
                    }
                    Rule::epoch => {
                        epoch = Some(u32::from_str(i.as_str())?);
                    }
                    Rule::version => {
                        version = Some(i.as_str().to_string());
                    }
                    Rule::EOI => break,
                    _ => unreachable!(),
                }
            }
        }
        if let (Some(image), Some(epoch), Some(version)) = (image, epoch, version) {
            Ok(ImageRef {
                image,
                epoch,
                version,
            })
        } else {
            anyhow::bail!("ImageRef parts not set")
        }
    }
}
