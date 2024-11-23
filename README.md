# splimer

I had a lot of huge files (especially games) that I wanted to store in Telegram, but Telegram has annoying limit that file size should be less 4 Gb, so I ended up creating a program that **spli**ts my .zip file and can **mer**ge it back to working state.

Because it is written is RUST, it should be 🤩🤩🤩BLAZINGLY FAST🤩🤩🤩

## How to compile it?

Firstly, you have to install `rustc` compiler and Cargo. Usually `rustup` installs `rustc` and Cargo for you:

* for Windows see this: https://rustup.rs/
* for Linux and MacOS use this command:

    ```shell
    curl https://sh.rustup.rs -sSf | sh
    ```

Then do this in terminal:

```shell
git clone https://github.com/pelmesh619/splimer.git
cd splimer
cargo build --release --target-dir="./"
```

If you are struggling with `linker 'cc' not found` error, try [this commands](https://stackoverflow.com/a/66598982) for Linux

Then you will get `splimer` executable file

Or if you are lazy WINDOWS user, you can just download it from "Releases" tab

## How to use it?🧐

Very simply, this is a syntax:

```
splimer
    (input_filename)                Input file name

    -S (memory-value)
    --fragment-size=(memory-value)  Size of one output fragment; can be float number
                                    with suffixes `b`, `m`, `k`, `g`,
                                    ex. `1m` or `1mb` is 1048576 bytes, 
                                    (by default is `1g`, 1073741824 bytes)

    -n (number)
    --parts=(number)                Number of output parts; should be more than 1.
                                    Makes all output files equal size.
                                    If entered `--fragment-size` will be ignored

    -s
    --split                         Splits file `input_filename`
                                    If file has `filename.ext` pattern there will be created
                                    `filename_[N].splm` files in `output_directory`
                                    (by default is true)

    -m
    --merge                         Merges files. For `input_filename` having `filename.ext` pattern
                                    program will search `filename_[N].splm` files in directory
                                    of `input_filename` and will try to merge them into `filename_[merged].ext`
                                    (by default false, ignores -n and -S arguments)

    -o (output_directory)
    --output-directory=(output_directory)   Output directory
                                            (by default it is directory, where input file lies)

    -h 
    --help                                  Show help message
```

TL;DR Use this to split:

```
splimer myfile -S 512g
```

and this to merge

```
splimer myfile --merge
```

## Some important notes

* This program will try overwrite and truncate all files that it is supposed to overwrite. Although they are only `.splm` and `_[merged].XXX` files, be aware

* If you what to rename your output files after a program's work, do it with all of them, otherwise, they will be ignored while merging

(i am just lazy to think about all those ploblems)

