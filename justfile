set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load := true

red4ext_sdk_dir := join(justfile_directory(), "deps", "RED4ext.SDK")
red4ext_sdk_build_dir := join(red4ext_sdk_dir, "build")
ninja_file := 'compile_commands.json'

# e.g. Ninja or Visual Studio 17 2022 (final project needs to be built with Visual Studio 17 2022)
[working-directory('deps\RED4ext.SDK')]
setup GENERATOR='Visual Studio 17 2022':
    cmake -S . -B build -G '{{ GENERATOR }}' -DCMAKE_EXPORT_COMPILE_COMMANDS=ON

# ⚠️ requires admin privilege
link:
    New-Item -Path '{{ join(justfile_directory(), ninja_file) }}' -ItemType SymbolicLink -Value '{{ join(red4ext_sdk_build_dir, ninja_file) }}'

clear:
    rm -Force '{{ join(red4ext_sdk_build_dir, "CMakeCache.txt") }}'
    rm -Recurse -Force '{{ join(red4ext_sdk_build_dir, "CMakeFiles") }}'
