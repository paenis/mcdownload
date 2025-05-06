build profile='debug' *args='':
    if [[ "{{profile}}" == "debug" ]]; then cargo build {{args}}; else cargo build --profile {{profile}} {{args}}; fi

test:
    cargo nextest run --release

clippy:
    cargo +nightly clippy

binsize *profile='debug release':
    @for i in {{profile}}; do \
        just build $i --quiet; \
    done
    @for i in {{profile}}; do \
        printf "$i: "; \
        echo "$(du -h target/$i/mcdl | cut -f1)" | column -t; \
    done
