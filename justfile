set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]
set dotenv-load

DEFAULT_GAME_DIR     := join("C:\\", "Program Files (x86)", "Steam", "steamapps", "common", "Cyberpunk 2077")

game_dir             := env_var_or_default("GAME_DIR", DEFAULT_GAME_DIR)
redscript_deploy_dir := join(game_dir, "r6", "scripts")
red4ext_deploy_dir   := join(game_dir, "red4ext", "plugins")
examples_dir         := join(justfile_directory(), "examples")

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
    @cp -Force '{{from}}' '{{to}}';
    @Write-Host 'Overwrite file: {{from}} => {{to}}';

# copy folder (overwriting)
[private]
overwrite-folder from to:
    @cp -Recurse -Force '{{ join(from, "*") }}' '{{to}}';
    @Write-Host 'Overwrite folder content: {{ join(from, "*") }} => {{to}}';

# list examples folders
[private]
examples:
    @(Get-ChildItem -Directory '{{ join(justfile_directory(), "examples") }}' | ?{ $_.PSIsContainer } | Select Name).Name

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
    @if (Test-Path '{{ join(dir, "reds") }}') { \
        just setup '{{ join(redscript_deploy_dir, capitalize(name)) }}'; \
        just overwrite-folder '{{ join(dir, "reds") }}' '{{ join(redscript_deploy_dir, capitalize(name)) }}'; \
    } else { Write-Host 'Skipping "reds" folder for {{dir}}' }

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
    @just examples | Foreach-Object { just reload ('{{examples_dir}}' + '\' + $_) $_ }

alias r := hot-reload

# install binaries from examples packages
install target='release':
    @just examples | Foreach-Object { \
        just build '{{target}}' ('{{examples_dir}}' + '\' + $_); \
        just setup ('{{red4ext_deploy_dir}}\' + $_); \
        just overwrite-file ('{{examples_dir}}' + '\' + $_ + '\' + 'target' + '\' + '{{target}}'+ '\' + $_ + '.dll') ('{{red4ext_deploy_dir}}\' + $_ + '\' + $_ + '.dll'); \
    }
    @just hot-reload

# @just examples | Foreach-Object { just examples/$_/install '{{target}}'; Write-Host '' }
alias i := install

# uninstall examples packages
uninstall:
    @just examples | Foreach-Object { \
        just delete ('{{red4ext_deploy_dir}}' + '\' + $_); \
        just delete ('{{redscript_deploy_dir}}' + '\' + $_.Substring(0,1).ToUpper() + $_.Substring(1)); \
        Write-Host ''; \
    }

# install examples packages (dev mode)
dev: (install 'debug')

alias d := dev

# display logs
logs:
    @just cat '{{ join(game_dir, "red4ext", "logs", "red4ext.log") }}'
    @just examples | Foreach-Object { just cat ('{{game_dir}}' + '\' + 'red4ext' + '\' + 'logs' + '\' + $_ + '.log'); Write-Host '' }

# lint files
lint:
    cargo +nightly fmt --all
    cargo fix --allow-dirty --allow-staged
    cargo clippy --fix --allow-dirty --allow-staged