use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::{HashMap, HashSet};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

fn collect_pdfs_recursive(dir: &Path, pdfs: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            collect_pdfs_recursive(&path, pdfs)?;
        } else {
            let is_pdf = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                .unwrap_or(false);
            if is_pdf {
                pdfs.push(path);
            }
        }
    }

    Ok(())
}

#[derive(Parser)]
#[command(name = "pdfer")]
#[command(
    about = "Merge and split PDFs from the command line",
    version,
    long_about = "A fast, safe, and portable PDF utility for developers and power users.\n\
                  \n\
                  Examples:\n\
                  • Quick info:  pdfer test.pdf\n\
                  • Merge:       pdfer merge a.pdf b.pdf -o out.pdf\n\
                  • Split:       pdfer split doc.pdf 1,3,5-10"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(global = true)]
    files: Vec<PathBuf>,

    #[arg(short, long, global = true)]
    info: bool,

    #[arg(short, long, global = true)]
    recursive: bool,
}

#[derive(Subcommand)]
enum Commands {
    #[command(
        visible_alias = "m",
        after_help = "Examples:\n  pdfer merge a.pdf b.pdf -o out.pdf\n  pdfer m *.pdf -o merged.pdf"
    )]
    Merge {
        #[arg(required = true)]
        inputs: Vec<PathBuf>,

        #[arg(short, long, default_value = "merged.pdf")]
        output: PathBuf,
    },

    #[command(
        visible_alias = "s",
        after_help = "Examples:\n  pdfer split document.pdf              # Split all pages\n  pdfer split report.pdf 1,3,5-10       # Split specific pages\n  pdfer s doc.pdf 5-                    # Split from page 5 to end"
    )]
    Split {
        input: PathBuf,
        #[arg(value_name = "PAGES")]
        pages: Option<String>,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(hide = true, trailing_var_arg = true)]
        extra_args: Vec<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.command.is_none() {
        if cli.files.is_empty() {
            bail!(
                "No PDF files or command specified. Try 'pdfer --help' or 'pdfer <file.pdf>' for quick info"
            );
        }

        let mut pdf_files = Vec::new();
        for path in &cli.files {
            if cli.recursive && path.is_dir() {
                collect_pdfs_recursive(path, &mut pdf_files)?;
            } else if path.is_file() {
                let is_pdf = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pdf"))
                    .unwrap_or(false);

                if is_pdf {
                    pdf_files.push(path.clone());
                } else {
                    bail!("Non-PDF file provided: {}", path.display());
                }
            } else if path.is_dir() {
                bail!(
                    "'{}' is a directory. Use -r/--recursive to search subdirectories",
                    path.display()
                );
            } else {
                bail!("Invalid path: {}", path.display());
            }
        }

        if pdf_files.is_empty() {
            bail!("No PDF files found");
        }

        let mut total_pages = 0;
        pdf_files.sort();
        for file in &pdf_files {
            match show_pdf_info(file) {
                Ok(page_count) => total_pages += page_count,
                Err(e) => eprintln!("Error reading {}: {}", file.display(), e),
            }
            if pdf_files.len() > 1 {
                println!();
            }
        }

        if pdf_files.len() > 1 {
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Total: {} PDF(s), {} page(s)", pdf_files.len(), total_pages);
        }

        return Ok(());
    }

    match cli.command.unwrap() {
        Commands::Merge { inputs, output } => {
            if cli.info {
                for input in &inputs {
                    let _ = show_pdf_info(input);
                    println!();
                }
            }
            merge_pdfs(&inputs, &output)?
        }
        Commands::Split {
            input,
            pages,
            output,
            extra_args,
        } => {
            if !extra_args.is_empty() {
                bail!(
                    "Split command accepts only ONE input PDF file.\n\
                     Found extra arguments: {}\n\
                     \n\
                     ⚠️ If your filename contains spaces, wrap it in quotes:\n\
                        pdfer split \"file with spaces.pdf\"\n\
                     \n\
                     Usage: pdfer split <file.pdf> [PAGES] [-o OUTPUT]\n\
                     \n\
                     To split multiple PDFs, run the command separately for each:\n\
                       pdfer split a.pdf\n\
                       pdfer split b.pdf",
                    extra_args.join(", ")
                );
            }

            if cli.info {
                let _ = show_pdf_info(&input);
                println!();
            }

            let output = output.unwrap_or_else(|| {
                let stem = input
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "split".to_string());
                PathBuf::from(format!("{}_pages", stem))
            });
            split_pdf(&input, &output, pages.as_deref())?
        }
    }
    Ok(())
}

fn show_pdf_info(path: &Path) -> Result<usize> {
    let doc =
        Document::load(path).with_context(|| format!("Failed to load PDF: {}", path.display()))?;

    let page_count = doc.get_pages().len();

    println!("📄 {}", path.display());
    println!("   Pages: {}", page_count);
    println!("   Version: {}", doc.version);

    if let Ok(info_dict) = doc.trailer.get(b"Info") {
        if let Object::Reference(id) = info_dict {
            if let Ok(Object::Dictionary(info)) = doc.get_object(*id) {
                if let Ok(Object::String(title, _)) = info.get(b"Title") {
                    println!("   Title: {}", String::from_utf8_lossy(title));
                }
                if let Ok(Object::String(author, _)) = info.get(b"Author") {
                    println!("   Author: {}", String::from_utf8_lossy(author));
                }
                if let Ok(Object::String(subject, _)) = info.get(b"Subject") {
                    println!("   Subject: {}", String::from_utf8_lossy(subject));
                }
            }
        }
    }

    let pages = doc.get_pages();
    let page_nums: Vec<u32> = pages.keys().copied().collect();
    if !page_nums.is_empty() {
        let mut sorted = page_nums.clone();
        sorted.sort_unstable();
        if sorted.len() <= 10 {
            println!("   Page numbers: {:?}", sorted);
        } else {
            println!(
                "   Page numbers: {} to {}",
                sorted[0],
                sorted[sorted.len() - 1]
            );
        }
    }

    Ok(page_count)
}

fn parse_page_ranges(spec: &str, total_pages: usize) -> Result<Vec<usize>> {
    if total_pages == 0 {
        bail!("PDF has no pages");
    }

    let mut pages = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }

        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() != 2 {
                bail!("Invalid range syntax: '{}'", part);
            }

            let start_str = bounds[0].trim();
            let end_str = bounds[1].trim();

            if start_str.is_empty() {
                bail!("Invalid range: '{}' (page numbers must be >= 1)", part);
            }

            let start = start_str
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("Invalid page number: '{}'", start_str))?;

            let end = if end_str.is_empty() {
                total_pages
            } else {
                end_str
                    .parse::<usize>()
                    .map_err(|_| anyhow::anyhow!("Invalid page number: '{}'", end_str))?
            };

            if start < 1 {
                bail!("Page numbers must be >= 1");
            }
            if start > total_pages {
                bail!(
                    "Start page {} is beyond document end ({})",
                    start,
                    total_pages
                );
            }
            if !end_str.is_empty() && end > total_pages {
                bail!("End page {} is beyond document end ({})", end, total_pages);
            }

            let actual_end = end.min(total_pages);
            if start <= actual_end {
                pages.extend(start..=actual_end);
            } else {
                bail!("Invalid range: '{}' (start > end)", part);
            }
        } else {
            let page = part
                .parse::<usize>()
                .map_err(|_| anyhow::anyhow!("Invalid page number: '{}'", part))?;
            if page < 1 || page > total_pages {
                bail!(
                    "Page {} is out of range (PDF has {} pages)",
                    page,
                    total_pages
                );
            }
            pages.push(page);
        }
    }

    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}

fn resolve_output_conflict(output: &Path, is_directory: bool) -> Result<Option<PathBuf>> {
    let current_output = output.to_path_buf();

    if !current_output.exists() {
        return Ok(Some(current_output));
    }

    let output_type = if is_directory { "directory" } else { "file" };
    print!(
        "⚠️ Output {} '{}' already exists. Action? (Y=overwrite, R=rename, N=abort): ",
        output_type,
        current_output.display()
    );
    io::stdout().flush()?;
    let mut choice = String::new();
    io::stdin().read_line(&mut choice)?;
    match choice.trim().to_lowercase().as_str() {
        "y" | "yes" => Ok(Some(current_output)),
        "n" | "no" => {
            println!("Aborted.");
            Ok(None)
        }
        "r" | "rename" => {
            let prompt = if is_directory {
                "Enter a new output folder: "
            } else {
                "Enter a new filename or folder (e.g., report.pdf or C:\\output): "
            };
            print!("{}", prompt);
            io::stdout().flush()?;
            let mut new_path = String::new();
            io::stdin().read_line(&mut new_path)?;
            let trimmed = new_path.trim();
            if trimmed.is_empty() {
                println!("Empty path. Aborted.");
                return Ok(None);
            }
            let mut new_output = PathBuf::from(trimmed);

            if !is_directory {
                if new_output.is_dir() {
                    if let Some(filename) = output.file_name() {
                        new_output.push(filename);
                    }
                }
            }

            if new_output.exists() {
                let msg = if is_directory {
                    format!(
                        "❌ Output folder '{}' already exists. Aborted to prevent overwrite.",
                        new_output.display()
                    )
                } else {
                    format!(
                        "❌ Output '{}' already exists. Aborted to prevent overwrite.",
                        new_output.display()
                    )
                };
                println!("{}", msg);
                return Ok(None);
            }

            Ok(Some(new_output))
        }
        _ => {
            println!("Invalid choice. Aborted.");
            Ok(None)
        }
    }
}

fn merge_pdfs(inputs: &[PathBuf], output: &Path) -> Result<()> {
    if inputs.is_empty() {
        bail!("No input files provided");
    }
    if inputs.len() == 1 {
        println!("⚠️ Note: Only one input file provided. This will copy/repair the PDF.");
    }

    let Some(current_output) = resolve_output_conflict(output, false)? else {
        return Ok(());
    };

    let current_output = match current_output.extension().and_then(|e| e.to_str()) {
        Some("pdf") => current_output,
        _ => current_output.with_extension("pdf"),
    };

    println!("Merging {} PDF(s)...", inputs.len());
    let mut merged = Document::with_version("1.5");
    let mut page_refs: Vec<Object> = Vec::new();

    for input in inputs {
        println!("  Processing: {}", input.display());
        let doc = Document::load(input)
            .with_context(|| format!("Failed to load PDF: {}", input.display()))?;
        if doc.get_pages().is_empty() {
            bail!("Input PDF has no pages: {}", input.display());
        }

        let mut id_map: HashMap<ObjectId, ObjectId> = HashMap::new();
        let mut new_id = merged.max_id + 1;

        for &old_id in doc.objects.keys() {
            id_map.insert(old_id, (new_id, 0));
            new_id += 1;
        }
        merged.max_id = new_id - 1;

        for (&old_id, obj) in doc.objects.iter() {
            let new_id = id_map[&old_id];
            let mut cloned = obj.clone();
            update_references_in_object(&mut cloned, &id_map)?;
            merged.objects.insert(new_id, cloned);
        }

        for (_, &page_id) in doc.get_pages().iter() {
            if let Some(&new_page_id) = id_map.get(&page_id) {
                page_refs.push(Object::Reference(new_page_id));
            }
        }
    }

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type".to_vec(), Object::Name(b"Pages".to_vec()));
    pages_dict.set(b"Kids".to_vec(), Object::Array(page_refs.clone()));
    pages_dict.set(b"Count".to_vec(), Object::Integer(page_refs.len() as i64));
    let pages_id = merged.add_object(pages_dict);

    let mut catalog = Dictionary::new();
    catalog.set(b"Type".to_vec(), Object::Name(b"Catalog".to_vec()));
    catalog.set(b"Pages".to_vec(), Object::Reference(pages_id));
    let catalog_id = merged.add_object(catalog);

    merged.trailer.set("Root", Object::Reference(catalog_id));
    merged
        .trailer
        .set("Size", Object::Integer(merged.max_id as i64 + 1));

    merged
        .save(&current_output)
        .with_context(|| format!("Failed to save: {}", current_output.display()))?;
    println!("✓ Merged PDF saved: {}", current_output.display());
    Ok(())
}

fn split_pdf(input: &Path, output: &Path, pages_spec: Option<&str>) -> Result<()> {
    if !input.exists() {
        bail!("Input file does not exist: {}", input.display());
    }
    if !input.is_file() {
        bail!("Input is not a file: {}", input.display());
    }

    let Some(current_output) = resolve_output_conflict(output, true)? else {
        return Ok(());
    };

    if current_output.exists() && current_output.is_dir() {
        for entry in std::fs::read_dir(&current_output)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("page_") && name.ends_with(".pdf") {
                    std::fs::remove_file(path)?;
                }
            }
        }
    }

    let doc = Document::load(input)
        .with_context(|| format!("Failed to load PDF: {}", input.display()))?;
    if doc.get_pages().is_empty() {
        bail!("Input PDF has no pages: {}", input.display());
    }
    let total_pages = doc.get_pages().len();
    println!("PDF has {} pages.", total_pages);

    let page_numbers = if let Some(mut spec) = pages_spec.map(|s| s.to_string()) {
        loop {
            match parse_page_ranges(&spec, total_pages) {
                Ok(pages) => break pages,
                Err(e) => {
                    println!("❌ Invalid page spec: {}", e);
                    print!("Enter pages to split (e.g., 1,3,5-7,10-): ");
                    io::stdout().flush()?;
                    let mut new_spec = String::new();
                    io::stdin().read_line(&mut new_spec)?;
                    spec = new_spec.trim().to_string();
                    if spec.is_empty() {
                        println!("Aborted.");
                        return Ok(());
                    }
                }
            }
        }
    } else {
        (1..=total_pages).collect()
    };

    if page_numbers.is_empty() {
        bail!("No pages to split (check your page range)");
    }

    std::fs::create_dir_all(&current_output).with_context(|| {
        format!(
            "Failed to create output directory: {}",
            current_output.display()
        )
    })?;

    let all_page_ids: Vec<ObjectId> = doc.page_iter().collect();

    let mut page_dependencies: HashMap<ObjectId, HashSet<ObjectId>> = HashMap::new();
    let mut cloned_objects: HashMap<ObjectId, Object> = HashMap::new();

    for &page_id in &all_page_ids {
        let mut referenced = HashSet::new();
        collect_referenced_objects(&doc, page_id, &mut referenced)?;
        page_dependencies.insert(page_id, referenced.clone());

        for &obj_id in &referenced {
            if obj_id == (0, 0) {
                continue;
            }
            if !cloned_objects.contains_key(&obj_id) {
                cloned_objects.insert(obj_id, doc.get_object(obj_id)?.clone());
            }
        }
    }

    if page_numbers.len() == total_pages && is_contiguous(&page_numbers) && page_numbers[0] == 1 {
        println!("Splitting all pages...");
    } else if page_numbers.len() <= 20 {
        println!("Splitting pages: {:?}", page_numbers);
    } else if is_contiguous(&page_numbers) {
        println!(
            "Splitting pages {} to {}",
            page_numbers[0],
            page_numbers[page_numbers.len() - 1]
        );
    } else {
        println!(
            "Splitting {} pages (including {}, {}, ..., {})",
            page_numbers.len(),
            page_numbers[0],
            page_numbers[1],
            page_numbers[page_numbers.len() - 1]
        );
    }

    for (idx, &page_num) in page_numbers.iter().enumerate() {
        if page_numbers.len() > 10 {
            print!("\rProcessing: {}/{}", idx + 1, page_numbers.len());
            io::stdout().flush()?;
        }

        let page_id = all_page_ids[page_num - 1];
        let referenced = &page_dependencies[&page_id];

        let mut single = Document::with_version("1.5");
        let mut id_map = HashMap::new();
        let mut new_id = 1u32;

        for &obj_id in referenced {
            if obj_id == (0, 0) {
                continue;
            }
            let cloned_obj = cloned_objects.get(&obj_id).unwrap();
            id_map.insert(obj_id, (new_id, 0));
            single.objects.insert((new_id, 0), cloned_obj.clone());
            new_id += 1;
        }
        single.max_id = new_id - 1;

        for obj in single.objects.values_mut() {
            update_references_in_object(obj, &id_map)?;
        }

        let new_page_id = id_map[&page_id];
        let mut pages_dict = Dictionary::new();
        pages_dict.set(b"Type".to_vec(), Object::Name(b"Pages".to_vec()));
        pages_dict.set(
            b"Kids".to_vec(),
            Object::Array(vec![Object::Reference(new_page_id)]),
        );
        pages_dict.set(b"Count".to_vec(), Object::Integer(1));
        let pages_id = single.add_object(pages_dict);

        if let Object::Dictionary(page_dict) = single.objects.get_mut(&new_page_id).unwrap() {
            page_dict.set(b"Parent".to_vec(), Object::Reference(pages_id));
        }

        let mut catalog = Dictionary::new();
        catalog.set(b"Type".to_vec(), Object::Name(b"Catalog".to_vec()));
        catalog.set(b"Pages".to_vec(), Object::Reference(pages_id));
        let catalog_id = single.add_object(catalog);

        single.trailer.set("Root", Object::Reference(catalog_id));
        single
            .trailer
            .set("Size", Object::Integer(single.max_id as i64 + 1));

        let out_path = current_output.join(format!("page_{:03}.pdf", page_num));
        single.save(&out_path).with_context(|| {
            format!("Failed to save page {} to {}", page_num, out_path.display())
        })?;
    }

    if page_numbers.len() > 10 {
        eprintln!();
    }
    eprintln!("✓ Done!");
    Ok(())
}

fn is_contiguous(pages: &[usize]) -> bool {
    if pages.len() <= 1 {
        return true;
    }
    for i in 1..pages.len() {
        if pages[i] != pages[i - 1] + 1 {
            return false;
        }
    }
    true
}

fn update_references_in_object(
    obj: &mut Object,
    id_map: &HashMap<ObjectId, ObjectId>,
) -> Result<()> {
    match obj {
        Object::Reference(id) => {
            if let Some(&new_id) = id_map.get(id) {
                *obj = Object::Reference(new_id);
            }
        }
        Object::Array(items) => {
            for item in items {
                update_references_in_object(item, id_map)?;
            }
        }
        Object::Dictionary(dict) => {
            let keys: Vec<_> = dict.iter().map(|(k, _)| k.clone()).collect();
            for key in keys {
                if let Ok(val) = dict.get_mut(&key) {
                    update_references_in_object(val, id_map)?;
                }
            }
        }
        Object::Stream(stream) => {
            let keys: Vec<_> = stream.dict.iter().map(|(k, _)| k.clone()).collect();
            for key in keys {
                if let Ok(val) = stream.dict.get_mut(&key) {
                    update_references_in_object(val, id_map)?;
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_referenced_objects(
    doc: &Document,
    obj_id: ObjectId,
    visited: &mut HashSet<ObjectId>,
) -> Result<()> {
    if !visited.insert(obj_id) {
        return Ok(());
    }

    let obj = doc.get_object(obj_id)?;
    collect_from_object(doc, obj, visited)?;
    Ok(())
}

fn collect_from_object(
    doc: &Document,
    obj: &Object,
    visited: &mut HashSet<ObjectId>,
) -> Result<()> {
    match obj {
        Object::Reference(id) => collect_referenced_objects(doc, *id, visited)?,
        Object::Array(items) => {
            for item in items {
                collect_from_object(doc, item, visited)?;
            }
        }
        Object::Dictionary(dict) => {
            for (_, val) in dict.iter() {
                collect_from_object(doc, val, visited)?;
            }
        }
        Object::Stream(stream) => {
            for (_, val) in stream.dict.iter() {
                collect_from_object(doc, val, visited)?;
            }
        }
        _ => {}
    }
    Ok(())
}
