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

Then you will get `splimer` executable file

## How to use it?


