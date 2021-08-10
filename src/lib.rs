use anyhow::{anyhow, bail, Context as _};
use camino::Utf8Path;
use cargo_metadata as cm;
use itertools::Itertools as _;
use std::{
    env,
    ffi::OsStr,
    fmt,
    io::{self, Write},
    path::{Path, PathBuf},
};
use structopt::{clap::AppSettings, StructOpt};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor as _};

#[derive(StructOpt)]
#[structopt(
    about,
    author,
    bin_name("cargo"),
    global_settings(&[AppSettings::DeriveDisplayOrder, AppSettings::UnifiedHelpMessage])
)]
pub enum Opt {
    #[structopt(
        about,
        author,
        usage(
            r#"cargo executable-payload [OPTIONS]
    cargo executable-payload [OPTIONS] --src <PATH>
    cargo executable-payload [OPTIONS] --bin <NAME>"#,
        )
    )]
    ExecutablePayload {
        /// Use `cross` instead of `$CARGO`
        #[structopt(long)]
        use_cross: bool,

        /// Path to `strip(1)`
        #[structopt(long, value_name("PATH"))]
        strip_exe: Option<PathBuf>,

        /// Do not apply `upx`
        #[structopt(long)]
        no_upx: bool,

        /// Write output to the file instead of stdout
        #[structopt(short, long, value_name("PATH"))]
        output: Option<PathBuf>,

        /// Path the main source file of the bin target
        #[structopt(long, value_name("PATH"), conflicts_with("bin"))]
        src: Option<PathBuf>,

        /// Name of the bin target
        #[structopt(long, value_name("NAME"))]
        bin: Option<String>,

        /// Build for the target triple
        #[structopt(long, value_name("TRIPLE"), default_value("x86_64-unknown-linux-musl"))]
        target: String,

        /// Path to Cargo.toml
        #[structopt(long, value_name("PATH"))]
        manifest_path: Option<PathBuf>,
    },
}

pub struct Shell {
    stderr: StandardStream,
}

impl Shell {
    pub fn new() -> Self {
        Self {
            stderr: StandardStream::stderr(if atty::is(atty::Stream::Stderr) {
                ColorChoice::Auto
            } else {
                ColorChoice::Never
            }),
        }
    }

    pub fn err(&mut self) -> &mut dyn Write {
        &mut self.stderr
    }

    pub(crate) fn status(
        &mut self,
        status: impl fmt::Display,
        message: impl fmt::Display,
    ) -> io::Result<()> {
        self.print(status, message, Color::Green, true)
    }

    pub fn error(&mut self, message: impl fmt::Display) -> io::Result<()> {
        self.print("error", message, Color::Red, false)
    }

    fn print(
        &mut self,
        status: impl fmt::Display,
        message: impl fmt::Display,
        color: Color,
        justified: bool,
    ) -> io::Result<()> {
        self.stderr
            .set_color(ColorSpec::new().set_bold(true).set_fg(Some(color)))?;
        if justified {
            write!(self.stderr, "{:>12}", status)?;
        } else {
            write!(self.stderr, "{}", status)?;
            self.stderr.set_color(ColorSpec::new().set_bold(true))?;
            write!(self.stderr, ":")?;
        }
        self.stderr.reset()?;
        writeln!(self.stderr, " {}", message)
    }
}

impl Default for Shell {
    fn default() -> Self {
        Self::new()
    }
}

pub fn run(opt: Opt, shell: &mut Shell) -> anyhow::Result<()> {
    let Opt::ExecutablePayload {
        use_cross,
        strip_exe,
        no_upx,
        output,
        src,
        bin,
        target,
        manifest_path,
    } = opt;

    let cwd = env::current_dir().with_context(|| "failed to get CWD")?;
    let manifest_path = if let Some(manifest_path) = manifest_path {
        cwd.join(manifest_path.strip_prefix(".").unwrap_or(&manifest_path))
    } else {
        locate_project(&cwd)?
    };
    let metadata = cargo_metadata(&manifest_path, &cwd)?;

    let (bin, bin_package) = if let Some(bin) = bin {
        bin_target_by_name(&metadata, &bin)
    } else if let Some(src) = src {
        bin_target_by_src_path(&metadata, &cwd.join(src))
    } else {
        exactly_one_bin_target(&metadata)
    }?;

    let source_code = std::fs::read_to_string(&bin.src_path)
        .with_context(|| format!("could not read `{}`", bin.src_path))?;

    let artifact_base64 = build(
        shell,
        &metadata.target_directory,
        &bin_package.manifest_path.with_file_name(""),
        &bin.name,
        use_cross,
        &target,
        strip_exe.map(|p| cwd.join(p)).as_deref(),
        no_upx,
    )?;

    let rs = format_with_template(&source_code, &artifact_base64);
    if let Some(output) = output {
        std::fs::write(output, rs)?;
    } else {
        let mut stdout = io::stdout();
        stdout.write_all(rs.as_ref())?;
        stdout.flush()?;
    }
    Ok(())
}

fn locate_project(cwd: &Path) -> anyhow::Result<PathBuf> {
    cwd.ancestors()
        .map(|p| p.join("Cargo.toml"))
        .find(|p| p.exists())
        .with_context(|| {
            format!(
                "could not find `Cargo.toml` in `{}` or any parent directory",
                cwd.display(),
            )
        })
}

fn cargo_metadata(manifest_path: &Path, cwd: &Path) -> cm::Result<cm::Metadata> {
    cm::MetadataCommand::new()
        .manifest_path(manifest_path)
        .current_dir(cwd)
        .exec()
}

fn bin_target_by_name<'a>(
    metadata: &'a cm::Metadata,
    name: &str,
) -> anyhow::Result<(&'a cm::Target, &'a cm::Package)> {
    match *bin_targets(metadata)
        .filter(|(t, _)| t.name == name)
        .collect::<Vec<_>>()
    {
        [] => bail!("no bin target named `{}`", name),
        [bin] => Ok(bin),
        [..] => bail!("multiple bin targets named `{}` in this workspace", name),
    }
}

fn bin_target_by_src_path<'a>(
    metadata: &'a cm::Metadata,
    src_path: &Path,
) -> anyhow::Result<(&'a cm::Target, &'a cm::Package)> {
    match *bin_targets(metadata)
        .filter(|(t, _)| t.src_path == src_path)
        .collect::<Vec<_>>()
    {
        [] => bail!(
            "`{}` is not the main source file of any bin targets in this workspace ",
            src_path.display(),
        ),
        [bin] => Ok(bin),
        [..] => bail!(
            "multiple bin targets which `src_path` is `{}`",
            src_path.display(),
        ),
    }
}

fn exactly_one_bin_target(metadata: &cm::Metadata) -> anyhow::Result<(&cm::Target, &cm::Package)> {
    match &*bin_targets(metadata).collect::<Vec<_>>() {
        [] => bail!("no bin target in this workspace"),
        [bin] => Ok(*bin),
        [bins @ ..] => bail!(
            "could not determine which binary to choose. Use the `--bin` option or `--src` option \
             to specify a binary.\n\
             available binaries: {}\n\
             note: currently `cargo-executable-payload` does not support the `default-run` manifest \
             key.",
            bins.iter()
                .map(|(cm::Target { name, .. }, _)| name)
                .format(", "),
        ),
    }
}

fn bin_targets(metadata: &cm::Metadata) -> impl Iterator<Item = (&cm::Target, &cm::Package)> {
    metadata
        .packages
        .iter()
        .filter(move |cm::Package { id, .. }| metadata.workspace_members.contains(id))
        .flat_map(|p| p.targets.iter().map(move |t| (t, p)))
        .filter(|(cm::Target { kind, .. }, _)| *kind == ["bin".to_owned()])
}

#[allow(clippy::too_many_arguments)]
fn build(
    shell: &mut Shell,
    target_dir: &Utf8Path,
    manifest_dir: &Utf8Path,
    bin_name: &str,
    use_cross: bool,
    target: &str,
    strip_exe: Option<&Path>,
    no_upx: bool,
) -> anyhow::Result<String> {
    fn run_command(
        shell: &mut Shell,
        cwd: &Utf8Path,
        program: impl AsRef<OsStr>,
        args: &[impl AsRef<OsStr>],
        before_spawn: fn(&mut duct::Expression),
    ) -> anyhow::Result<()> {
        let program = program.as_ref();
        let program = which::which_in(&program, env::var_os("PATH"), cwd)
            .map_err(|_| anyhow!("`{}` does not seem to exist", program.to_string_lossy()))?;
        let args = args.iter().map(AsRef::as_ref).collect::<Vec<_>>();

        let format = format!(
            "`{}{}`",
            shell_escape::escape(program.to_string_lossy()),
            args.iter().format_with("", |arg, f| f(&format_args!(
                " {}",
                shell_escape::escape(arg.to_string_lossy()),
            ))),
        );

        shell.status("Running", &format)?;
        let mut cmd = duct::cmd(program, args).dir(cwd);
        before_spawn(&mut cmd);
        cmd.run()
            .with_context(|| format!("{} didn't exit successfully", format))?;
        Ok(())
    }

    let tempdir = tempfile::Builder::new()
        .prefix("cargo-executable-payload-")
        .tempdir()?;

    let program = if use_cross {
        "cross".into()
    } else {
        env::var_os("CARGO").with_context(|| "`$CARGO` is not present")?
    };
    let args = vec![
        OsStr::new("build"),
        OsStr::new("--release"),
        OsStr::new("--bin"),
        OsStr::new(bin_name),
        OsStr::new("--target"),
        OsStr::new(target),
    ];
    run_command(shell, manifest_dir, program, &args, |_| ())?;

    let mut artifact_path = target_dir.join(target).join("release").join(bin_name);
    if target.contains("windows") {
        artifact_path.set_extension("exe");
    }

    let artifact_file_name = artifact_path.file_name().unwrap_or("");

    std::fs::copy(&artifact_path, tempdir.path().join(artifact_file_name))?;

    let artifact_path = tempdir.path().join(artifact_file_name);

    let program = strip_exe.unwrap_or_else(|| "strip".as_ref());
    if let Ok(program) = which::which_in(program, env::var_os("PATH"), manifest_dir) {
        let args = [OsStr::new("-s"), artifact_path.as_ref()];
        run_command(shell, manifest_dir, program, &args, |_| ())?;
    }

    if !no_upx {
        if let Ok(program) = which::which_in("upx", env::var_os("PATH"), manifest_dir) {
            let args = [OsStr::new("--best"), artifact_path.as_ref()];
            run_command(shell, manifest_dir, program, &args, |cmd| {
                *cmd = cmd.stdout_to_stderr();
            })?;
        }
    }

    let artifact = std::fs::read(artifact_path)?;
    let artifact = base64::encode(artifact);

    tempdir.close()?;
    Ok(artifact)
}

fn format_with_template(original_source_code: &str, payload: &str) -> String {
    format!(
        r#"//! This code is generated by [cargo-executable-payload](https://github.com/qryxip/cargo-executable-payload).
//!
//! # Original source code
//!
//! ```
{original_source_code}//! ```

use std::{{
    fs::{{File, Permissions}},
    io::{{self, Write as _}},
    os::unix::{{fs::PermissionsExt as _, process::CommandExt as _}},
    process::Command,
}};

fn main() -> io::Result<()> {{
    let mut file = File::create(PATH)?;
    file.write_all(&decode())?;
    file.set_permissions(Permissions::from_mode(0o755))?;
    file.sync_all()?;
    drop(file);
    Err(Command::new(PATH).exec())
}}

fn decode() -> Vec<u8> {{
    let mut table = [0; 256];
    for (i, &c) in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
        .iter()
        .enumerate()
    {{
        table[usize::from(c)] = i as u8;
    }}

    let mut acc = vec![];

    for chunk in PAYLOAD.as_bytes().chunks(4) {{
        let index0 = table[usize::from(chunk[0])];
        let index1 = table[usize::from(chunk[1])];
        let index2 = table[usize::from(chunk[2])];
        let index3 = table[usize::from(chunk[3])];
        acc.push((index0 << 2) + (index1 >> 4));
        acc.push((index1 << 4) + (index2 >> 2));
        acc.push((index2 << 6) + index3);
    }}

    if PAYLOAD.ends_with("==") {{
        acc.pop();
        acc.pop();
    }} else if PAYLOAD.ends_with('=') {{
        acc.pop();
    }}

    acc
}}

static PATH: &str = "/tmp/a.out";
static PAYLOAD: &str = "{payload}";
"#,
        original_source_code = original_source_code
            .lines()
            .map(|line| match line {
                "" => "//!\n".to_owned(),
                line => format!("//! {}\n", line),
            })
            .join(""),
        payload = payload,
    )
}
