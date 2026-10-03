// Verify real assets in the executable without launching the webview.
import {readdirSync,readFileSync,statSync} from 'node:fs';
import {join} from 'node:path';
import {createHash} from 'node:crypto';
import {brotliDecompressSync} from 'node:zlib';
const binaryPath='src/target/release/bos-settings';
const binary=readFileSync(binaryPath);
const hash=bytes=>createHash('sha256').update(bytes).digest('hex');
const embedded=new Set();
for(const directory of readdirSync('src/target/release/build')){
 if(!directory.startsWith('bos-settings-'))continue;
 const root=join('src/target/release/build',directory,'out/tauri-codegen-assets');
 let files;try{files=readdirSync(root);}catch{continue;}
 for(const file of files){
  const bytes=readFileSync(join(root,file));
  if(binary.indexOf(bytes)===-1)continue;
  try{embedded.add(hash(brotliDecompressSync(bytes)));}catch{ /* Other build outputs are not compressed assets. */ }
 }
}
const walk=root=>readdirSync(root).flatMap(name=>{const file=join(root,name);return statSync(file).isDirectory()?walk(file):[file];});
// Tauri rewrites HTML to inject its bootstrap and CSP. Check generated code,
// styles, font files and license files against their original bytes.
const assets=walk('frontend/build').filter(file=>/\.(js|css|json|ttf|txt)$/.test(file));
const unmatched=assets.filter(file=>!embedded.has(hash(readFileSync(file))));
console.log(JSON.stringify({binary:binaryPath,sha256:hash(binary),assets_checked:assets.length,unmatched},null,2));
if(!assets.length||unmatched.length)process.exitCode=1;
