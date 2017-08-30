#![feature(ascii_ctype)]
#![feature(box_syntax)]
#![feature(catch_expr)]
#![allow(unused_parens)]

#[macro_use] extern crate clap;
#[macro_use] extern crate closet;
#[macro_use] extern crate error_chain;
#[macro_use] extern crate if_chain;
extern crate ignore;
extern crate itertools;
#[macro_use] extern crate lazy_static;
extern crate num_cpus;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

mod types;

use std::{cmp, env, fs};
use std::ascii::AsciiExt;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process;
use std::result::Result as StdResult;
use std::sync::{Arc, Mutex};
use ignore::{WalkBuilder, WalkState};
use ignore::types::TypesBuilder;
use itertools::Itertools;

error_chain! {
    errors {
        Many(errors: Vec<Box<Error>>) {
            description("some errors occurred")
            display("some errors occured:\n\t{}", errors.iter().map(|e| format!("{}", e)).join("\n\t"))
        }

        Bad(i: String, error: Box<Error>) {
            description("something bad happened with a particular file")
            display("[{}] {}", i, error)
        }
    }

    foreign_links {
        Io(io::Error);
        Ignore(ignore::Error);
        Serde(serde_json::Error);
    }
}

trait ChainKind {
    type Result;
    fn chain_kind<P, Q: Into<P>>(self, param: Q, kind: fn(P, Box<Error>) -> ErrorKind) -> Self::Result;
}
impl<T, E: Into<Error>> ChainKind for StdResult<T, E> {
    type Result = Result<T>;
    fn chain_kind<P, Q: Into<P>>(self, param: Q, kind: fn(P, Box<Error>) -> ErrorKind) -> Self::Result {
        self.map_err(|e| Error::from_kind(kind(param.into(), box e.into())))
    }
}

const DATADIR: &str = "/home/haptics/shared/Projects/Proton Pack/Surface Texture Bible Images";

fn main() {
    if let Err(e) = try_main() {
        println!("error: {}", e);

        for e in e.iter().skip(1) {
            println!("caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            println!("backtrace: {:?}", backtrace);
        }

        process::exit(1);
    }
}

fn try_main() -> Result<()> {
    let matches = clap_app!(extract =>
        (version: crate_version!())
        (author: crate_authors!())
        (about: crate_description!())
        (@arg IMAGE: "Image identifier (e.g. 0230_017)")
    ).get_matches();

    if let Some(imgid) = matches.value_of("IMAGE") {
        extract(imgid, None)
    } else {
        const BLACKLIST: &[&str] = &[
            "0230_038",
            "0230_042",
            "0230_048",
        ];

        let errors = Arc::new(Mutex::new(vec![]));
        WalkBuilder::new(DATADIR)
            .standard_filters(false)
            .threads(num_cpus::get())
            .types(TypesBuilder::new()
                   .add_defaults()
                   .select("txt")
                   .build().unwrap())
            .build_parallel()
            .run(clone_army!([errors]
                 move || box clone_army!([errors]
                                    move |dirent| -> WalkState {
                                        match (do catch {
                                            let dirent = dirent?;
                                            if dirent.metadata()?.is_dir() {
                                                if dirent.path() == Path::new(DATADIR) {
                                                    return WalkState::Continue;
                                                } else {
                                                    return WalkState::Skip;
                                                }
                                            }
                                            let imgid = dirent
                                                .path()
                                                .file_stem().ok_or("no file name")?
                                                .to_str().ok_or("non-UTF8 file stem")?;
                                            if !BLACKLIST.contains(&imgid) {
                                                extract(imgid,
                                                        Some(&Path::new(".")
                                                                  .with_file_name(dirent.file_name())
                                                                  .with_extension("extract.txt")))
                                                    .chain_kind(imgid, ErrorKind::Bad)
                                            } else {
                                                Ok(())
                                            }
                                        }) {
                                            Ok(_) => WalkState::Continue,
                                            Err(e) => {
                                                errors.lock().unwrap().push(box e);
                                                WalkState::Continue
                                            }
                                        }
                                    })));
        let errors = Arc::try_unwrap(errors).unwrap().into_inner().unwrap();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(Error::from_kind(ErrorKind::Many(errors)))
        }
    }
}

fn extract(imgid: &str, path: Option<&Path>) -> Result<()> {
    let result = do catch {
        println!("Extracting {}", imgid);
        let mut out: Box<Write> = if let Some(path) = path {
            Box::new(File::create(path).chain_kind(imgid, ErrorKind::Bad)?)
        } else {
            Box::new(io::stdout())
        };

        let mut json_path = Path::new(DATADIR).to_owned();
        json_path.push(imgid);
        json_path.set_extension("txt");
        let json: types::Annotations = serde_json::from_reader(File::open(json_path)?)?;

        macro_rules! close_enough {
            (|$text:ident| $key:expr) => {
                |t1, t2| {
                    let k1 = { let $text = t1; $key };
                    let k2 = { let $text = t2; $key };
                    if (k1 - k2).abs() <= 50 {
                        cmp::Ordering::Equal
                    } else {
                        k1.cmp(&k2)
                    }
                }
            }
        }

        let mut texts = json.text_annotations.clone();

        let (levels, ends, n);
        if texts.iter().any(|text| text.description == "BUYER") {
            levels = vec![1450, 3190, 4420];
            n = 6;
        } else {
            levels = vec![1130, 1480, 2550, 2900, 3970, 4300, 4420, 4790];
            n = 9;
        };
        ends = vec![450, 940, 1220, 1980, 2280, 3025, 3160];

        if env::var("DEBUG").is_ok() {
            for text in &texts {
                println!("{}", text);
            }
        }
        texts.retain(|text|    text.bounding_poly.width() <= 500
                            && levels.iter().any(|y| (y - (text.bounding_poly.top() as i32)).abs() <= 200)
                            && !text.description.chars().all(|c| c.is_uppercase() || c.is_numeric())
                            && !["Supplier", "No", ":", "|", ".", "Mr", "lo"].contains(&&text.description[..]));
        texts.sort_by(close_enough!(|text| text.bounding_poly.left()));
        texts.sort_by(close_enough!(|text| text.bounding_poly.top()));
        let mut merged_texts = texts.into_iter()
            .coalesce(|a, b| {
                if env::var("DEBUG").is_ok() {
                    println!("coalesce\n\t{}\n\t{}\n\t{}\t{}", a, b, ((a.bounding_poly.top() as i32) - (b.bounding_poly.top() as i32)).abs(), ((b.bounding_poly.left() as i32) - (a.bounding_poly.right() as i32)).abs());
                }
                if ((a.bounding_poly.top() as i32) - (b.bounding_poly.top() as i32)).abs() <= 100 {
                    let dist = ((b.bounding_poly.left() as i32) - (a.bounding_poly.right() as i32)).abs();
                    if dist <= 100 {
                        Ok(types::Annotation {
                            description: format!("{}{}{}",
                                                 a.description,
                                                 if dist <= 10 {
                                                     ""
                                                 } else {
                                                     " "
                                                 },
                                                 b.description),
                            bounding_poly: a.bounding_poly.merge(&b.bounding_poly),
                            locale: a.locale.clone(),
                        })
                    } else {
                        Err((a, b))
                    }
                } else {
                    Err((a, b))
                }
            })
            .collect::<Vec<_>>();

        merged_texts.retain(|text| text.description.len() <= 50
                                && ends.iter().any(|x| (x - (text.bounding_poly.right() as i32)).abs() <= 200));

        if merged_texts.len() != n+1 {
            // try filtering using the dictionary
            lazy_static! {
                static ref WORDS: Vec<String> = {
                    let words_file = BufReader::new(File::open("/usr/share/dict/words").unwrap());
                    let mut words = words_file.lines()
                                              .collect::<StdResult<Vec<_>,_>>().unwrap();
                    for extra in &[
                        "handpainted",
                        "stardust",
                    ] {
                        words.push(extra.to_string());
                    }
                    words
                };
            }

            merged_texts.retain(|text| text
                                        .description.split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
                                                    .filter(|s| !s.is_ascii_punctuation())
                                                    .map(str::to_lowercase)
                                                    .all(|word| WORDS.iter()
                                                                     .find(|entry| &entry[..] == word)
                                                                     .is_some()));
        }

        if merged_texts.len() == n+1 {
            merged_texts.sort_by_key(|text| text.bounding_poly.left());
            merged_texts.sort_by_key(|text| text.bounding_poly.top());
            let section = &merged_texts[n].description;
            for (i, text) in merged_texts[..n].iter().enumerate() {
                writeln!(out, "{} - {}", section, text.description)?;

                let mut inpath = Path::new(DATADIR).to_owned();
                inpath.push(imgid);
                inpath.set_extension("rot.png");
                let mut outpath = Path::new(".").to_owned();
                outpath.push(imgid);
                outpath.set_extension(format!("crop.{}.png", i));
                let status = process::Command::new("gm")
                    .arg("convert")
                    .arg(inpath)
                    .arg("-crop")
                    .arg(format!("960x1100+{}+{}", text.bounding_poly.right() - 850, text.bounding_poly.bottom() - 1160))
                    .args(&["-resize", "25%"])
                    .arg(outpath)
                    .status()?;
                if !status.success() {
                    Err("graphicsmagick failed")?;
                }
            }
        }

        Ok(())
    };

    if_chain! {
        if let Err(_) = result;
        if let Some(path) = path;
        if let Err(e) = fs::remove_file(path);
        then {
            result.chain_err(|| ErrorKind::Io(e))
        } else {
            result
        }
    }
}

