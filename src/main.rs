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
    ffi::{OsStr, OsString},
    path::Path,
};

const ALLOWED_EXTENSIONS: [&str; 4] = ["js", "jsx", "ts", "tsx"];
const PATTERNS: [&str; 4] = ["from \"", "from '", "import \"", "import '"];

struct RootEntry {
    file_name: String,
    path: OsString,
}

impl RootEntry {
    fn new(file_name: String, path: OsString) -> RootEntry {
        RootEntry { file_name, path }
    }
}

#[allow(unused_must_use)]
fn main() {
    let _args = args().skip(1);
    let args_length = _args.len();

    if args_length < 2 {
        panic!("Insufficient number of arguments provided.")
    }

    let mut args: Vec<String> = Vec::with_capacity(args_length);
    args.extend(_args);

    let filters = &args[2..];

    let root_entries = std::fs::read_dir(&args[0])
        .unwrap()
        .filter_map(|d| {
            // storing given path root entries
            if d.is_ok() {
                let dir_ok = d.ok().unwrap();
                if !filters.contains(&dir_ok.file_name().into_string().unwrap()) {
                    Some(RootEntry::new(
                        dir_ok.file_name().into_string().unwrap(),
                        dir_ok.path().into_os_string(),
                    ))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect::<Vec<RootEntry>>();

    for root_entry in &root_entries {
        read_dir_recursively(&root_entry.path, &args[1], &root_entries);
    }
}

#[allow(unused_must_use)]
fn read_dir_recursively<P>(
    path: P,
    alias: &String,
    root_entries: &Vec<RootEntry>,
) -> Result<(), std::io::Error>
where
    P: AsRef<Path>,
{
    let directories = std::fs::read_dir(path)?
        .filter_map(|d| d.ok())
        .collect::<Vec<_>>();
    for d in directories {
        let dir_metadata = d.metadata().unwrap();
        if dir_metadata.is_dir() {
            read_dir_recursively(d.path(), alias, root_entries);
        } else if dir_metadata.is_file() {
            let file_name = d.file_name();
            let extension = Path::new(&file_name)
                .extension()
                .and_then(OsStr::to_str)
                .unwrap();
            if ALLOWED_EXTENSIONS.contains(&extension) {
                inject(d.path(), alias, root_entries);
            }
        }
    }

    Ok(())
}

#[allow(unused_must_use)]
fn inject<P>(path: P, alias: &String, root_entries: &Vec<RootEntry>) -> ()
where
    P: AsRef<Path>,
{
    for entry in root_entries {
        let mut content = std::fs::read_to_string(&path).unwrap();
        for pattern in PATTERNS.iter() {
            let matcher = format!(
                "{pattern}{filename}",
                pattern = pattern.to_string(),
                filename = entry.file_name
            );
            let destination = format!(
                "{pattern}{alias}/{filename}",
                pattern = pattern.to_string(),
                alias = alias,
                filename = entry.file_name
            );
            content = content.replace(&matcher, &destination);
        }

        std::fs::write(&path, content);
    }
}
