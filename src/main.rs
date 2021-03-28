/*********
Usage ->

 - cargo run {path_to_src} {alias} [..filterable_dirs]
 - e.g cargo run ../myproject/src @app handlebars donottraverse public

Running this script might prompt you to allow many files to be opened simultaneously ->
    In this case, run this beforehand :

$ echo kern.maxfiles=65536 | sudo tee -a /etc/sysctl.conf
$ echo kern.maxfilesperproc=65536 | sudo tee -a /etc/sysctl.conf
$ sudo sysctl -w kern.maxfiles=65536
$ sudo sysctl -w kern.maxfilesperproc=65536
$ ulimit -n 65536

(65536 could be bumped higher although I don't recommend it, if the error still occurs, I suggest you run this program progressively over the codebase by using filter cli args)

To revert (apple default values) :

$ echo kern.maxfiles=12288 | sudo tee -a /etc/sysctl.conf
$ echo kern.maxfilesperproc=10240 | sudo tee -a /etc/sysctl.conf
$ sudo sysctl -w kern.maxfiles=12288
$ sudo sysctl -w kern.maxfilesperproc=10240
$ ulimit -n 12288

*********/

use std::{
    env::args,
    io::Result as IOResult,
    path::{Path, PathBuf},
};

const ALLOWED_EXTENSIONS: [&str; 4] = ["js", "jsx", "ts", "tsx"];
const PATTERNS: [&str; 4] = ["from \"", "from '", "import \"", "import '"];

struct RootEntry {
    file_name: String,
    path: PathBuf,
}

impl RootEntry {
    fn new(file_name: String, path: PathBuf) -> RootEntry {
        RootEntry { file_name, path }
    }
}

fn main() -> IOResult<()> {
    let args: Vec<_> = args().skip(1).collect();

    let (src_path, alias, filters) = match args.as_slice() {
        [src_path, alias, filters @ ..] => (src_path, alias, filters),
        _ => panic!("Insufficient number of arguments provided."),
    };

    let root_entries = std::fs::read_dir(src_path)?
        .filter_map(|entry_res| match entry_res {
            Ok(entry) => {
                let file_name = entry.file_name().into_string().ok()?;
                let bypass_filters = !filters.contains(&file_name);
                bypass_filters.then(|| Ok(RootEntry::new(file_name, entry.path())))
            }
            Err(e) => Some(Err(e)),
        })
        .collect::<IOResult<Vec<_>>>()?;

    for root_entry in &root_entries {
        read_dir_recursively(&root_entry.path, alias, &root_entries)?;
    }

    Ok(())
}

fn read_dir_recursively<P>(path: P, alias: &str, root_entries: &[RootEntry]) -> IOResult<()>
where
    P: AsRef<Path>,
{
    let directories = std::fs::read_dir(path)?;

    for dir_entry_res in directories {
        let dir_entry = dir_entry_res?;
        let metadata = dir_entry.metadata()?;
        let path = dir_entry.path();

        if metadata.is_dir() {
            read_dir_recursively(path, alias, root_entries)?;
        } else if metadata.is_file() {
            if let Some(extension) = path.extension() {
                let is_allowed_extension = ALLOWED_EXTENSIONS
                    .iter()
                    .any(|&allowed| allowed == extension);
                if is_allowed_extension {
                    inject(path, alias, root_entries)?;
                }
            }
        }
    }

    Ok(())
}

fn inject<P>(path: P, alias: &str, root_entries: &[RootEntry]) -> IOResult<()>
where
    P: AsRef<Path>,
{
    let mut content = std::fs::read_to_string(&path)?;

    for entry in root_entries {
        for pattern in PATTERNS.iter() {
            let matcher = format!(
                "{pattern}{filename}",
                pattern = pattern,
                filename = entry.file_name
            );
            let destination = format!(
                "{pattern}{alias}/{filename}",
                pattern = pattern,
                alias = alias,
                filename = entry.file_name
            );
            content = content.replace(&matcher, &destination);
        }
    }

    std::fs::write(&path, content)
}
