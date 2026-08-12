const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

const TUTORIAL_DIR = 'D:/Git/TauriProjects/RustWeb-Vue/docs/tutorial';
const OUTPUT_TYP   = path.join(__dirname, 'combined.typ');
const OUTPUT_PDF   = 'D:/Git/TauriProjects/RustWeb-Vue/docs/tutorial.pdf';
const TYPST_BIN    = 'C:/Users/dch/.cargo/bin/typst.exe';

function collectMarkdownFiles(dir) {
  const files = fs.readdirSync(dir);
  return files
    .filter(f => f.endsWith('.md'))
    .sort((a, b) => {
      const na = parseInt(a.split('-')[0]);
      const nb = parseInt(b.split('-')[0]);
      if (Number.isNaN(na) && Number.isNaN(nb)) return 0;
      if (Number.isNaN(na)) return 1;
      if (Number.isNaN(nb)) return -1;
      return na - nb;
    })
    .map(f => path.join(dir, f));
}

function readAllFiles(files) {
  let combined = '';
  for (const f of files) {
    const content = fs.readFileSync(f, 'utf-8');
    combined += content + '\n\n';
  }
  return combined;
}

function generate() {
  const files = collectMarkdownFiles(TUTORIAL_DIR);
  console.log(`Found ${files.length} markdown files`);

  const combinedMd = readAllFiles(files);
  const mdPath = path.join(__dirname, 'combined.md');
  fs.writeFileSync(mdPath, combinedMd, 'utf-8');

  execSync(`pandoc "${mdPath}" -o "${OUTPUT_TYP}" --to typst -f markdown-citations`, { stdio: 'inherit' });

  const typContent = fs.readFileSync(OUTPUT_TYP, 'utf-8')
    .replace(/#horizontalrule/g, '#line()');

  const preamble = `#set text(lang: "zh", font: "Microsoft YaHei", size: 11pt)
#set page(
  paper: "a4",
  margin: (top: 20mm, bottom: 20mm, left: 25mm, right: 25mm),
  numbering: "1 / 1",
  number-align: center,
)
#set heading(numbering: "1.1")
#show heading: set text(font: "Microsoft YaHei")
#show raw: set text(font: "Consolas", size: 9pt)

#outline(
  title: [目录],
  indent: auto,
)

`;

  const fullTyp = preamble + typContent;
  fs.writeFileSync(OUTPUT_TYP, fullTyp, 'utf-8');

  console.log('Compiling with typst...');
  execSync(`"${TYPST_BIN}" compile "${OUTPUT_TYP}" "${OUTPUT_PDF}"`, { stdio: 'inherit' });

  const stats = fs.statSync(OUTPUT_PDF);
  console.log(`PDF generated: ${OUTPUT_PDF} (${(stats.size / 1024).toFixed(0)} KB)`);
}

generate();
