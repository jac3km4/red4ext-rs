set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load

DEFAULT_GAME_DIR   := join("C:\\", "Program Files (x86)", "Steam", "steamapps", "common", "Cyberpunk 2077")

game_dir           := env_var_or_default("GAME_DIR", DEFAULT_GAME_DIR)
redscript_game_dir := join(game_dir, "r6", "scripts")

# list all commands
_default:
  @just --list --unsorted
  @echo "⚠️  on Windows, paths defined in .env must be double-escaped:"
  @echo 'e.g. GAME_DIR="C:\\\\path\\\\to\\\\your\\\\game\\\\folder"'

# create dir if not exists
[private]
setup path:
    @if (Test-Path '{{path}}') { Write-Host "Folder {{path}} already exist"; } else { New-Item '{{path}}' -ItemType Directory; }

# force delete folder
[private]
delete path:
    @if (Test-Path '{{path}}') { Write-Host "Delete folder: {{path}}"; Remove-Item -Recurse -Force '{{path}}'; }

# copy file (overwriting)
[private]
overwrite-file from to:
    cp -Force '{{from}}' '{{to}}';

# copy folder (overwriting)
[private]
overwrite-folder from to:
    cp -Recurse -Force '{{ join(from, "*") }}' '{{to}}';

# build plugin binary (pre-launch only)
[private]
build target='release' dir='{{justfile_directory()}}':
    @if (-NOT('{{target}}' -EQ 'debug') -AND -NOT('{{target}}' -EQ 'release')) { \
        Write-Host "target can only be 'debug' or 'release' (default to 'release')"; exit 1; \
    }
    @$manifest = '{{ join(dir, "Cargo.toml") }}'; \
    Write-Host "Build package: {{dir}}"; \
    if ('{{target}}' -EQ 'debug') { \
        cargo +nightly build --manifest-path $manifest; \
    } else { cargo +nightly build --manifest-path $manifest --release; }

# overwrite scripts (supports hot-reload)
[private]
reload dir name:
    @just setup '{{ join(redscript_game_dir, capitalize(name)) }}'
    @just overwrite-folder '{{ join(dir, "reds") }}' '{{ join(redscript_game_dir, capitalize(name)) }}'

# display file content
[private]
cat path:
    @if (Test-Path '{{path}}') { \
        if ((Get-Command bat).Path) { \
            bat --paging=never '{{ join(game_dir, "red4ext", "logs", "red4ext.log") }}'; \
        } else { \
            Write-Host "-----------------------"; \
            Write-Host "{{file_name(path)}}";  \
            Write-Host "-----------------------"; \
            type '{{path}}'; \
        } \
    } else { \
        Write-Host "-----------------------"; \
        Write-Host "{{file_name(path)}}: missing file"; \
        Write-Host "-----------------------"; \
    }

# install scripts from examples packages
hot-reload:
    @just examples/hello_world/hot-reload
    @just examples/menu_button/hot-reload
    @just examples/player_info/hot-reload

alias r := hot-reload

# install binaries from examples packages
install target='release':
    @just examples/hello_world/install '{{target}}'
    @just examples/menu_button/install '{{target}}'
    @just examples/player_info/install '{{target}}'
    @just hot-reload

alias i := install

# uninstall examples packages
uninstall:
    @just examples/hello_world/uninstall
    @just examples/menu_button/uninstall
    @just examples/player_info/uninstall

# install examples packages (dev mode)
dev: (install 'debug')

alias d := dev

# display logs
logs:
    @just cat '{{ join(game_dir, "red4ext", "logs", "red4ext.log") }}'
    @just examples/hello_world/logs
    @just examples/menu_button/logs
    @just examples/player_info/logs

# lint files
lint:
    cargo +nightly fmt --all
    cargo fix --allow-dirty --allow-staged
    cargo clippy --fix --allow-dirty --allow-staged