import re
import os

ver = re.search(r'\[package\]\nname = "splimer"\nversion = "(.+)"', open("Cargo.toml").read()).group(1)
commands = [
    ("cargo build --release --target x86_64-pc-windows-msvc",       'win10', 'x86', '64'),
    ("cargo zigbuild --release --target x86_64-unknown-linux-gnu",  'linux', 'x86', '64'),
    ("cargo zigbuild --release --target x86_64-apple-darwin",       'macos', 'x86', '64'),
    ("cargo build --release --target i686-pc-windows-msvc",         'win10', 'x86', '32'),
    ("cargo zigbuild --release --target i686-unknown-linux-gnu",    'linux', 'x86', '32'),
    ("cargo zigbuild --release --target aarch64-unknown-linux-gnu", 'linux', 'arm', '64'),
    ("cargo zigbuild --release --target aarch64-apple-darwin",      'macos', 'arm', '64'),
    ("cargo zigbuild --release --target aarch64-pc-windows-gnullvm", 'win10', 'arm', '64'),
]

for com, system, arch, bit in commands:
    print("$", com)
    os.system(com)

    target_name = com.split()[-1]
    
    if not os.path.exists('build'):
        os.mkdir("build")
    
    os.replace(
        f"./target/{target_name}/release/splimer{'.exe' if 'windows' in target_name else ''}",
        f"./build/splimer_v{ver}_{system}_{arch}_{bit}{'.exe' if 'windows' in target_name else ''}",
    )
