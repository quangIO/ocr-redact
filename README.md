## Run
```shell
sh ./download-model.sh
cargo build --release
```
You will need the libpdfium in the /usr/lib/ folder
https://github.com/bblanchon/pdfium-binaries/releases

Please use the release build. Debug builds are super slow

## Example usage
```shell
cargo run --release -- -p ~/Documents/somedoc.pdf -o /tmp/redacted -r "sensitive info"
```