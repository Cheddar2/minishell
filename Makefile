NAME    = minishell
CARGO   = cargo
PROFILE = release

all: $(NAME)

$(NAME): Cargo.toml Cargo.lock $(shell find src -name '*.rs')
	$(CARGO) build --$(PROFILE) --locked
	cp target/$(PROFILE)/$(NAME) ./$(NAME)

clean:
	$(CARGO) clean

fclean: clean
	rm -f $(NAME)

re: fclean all

test:
	$(CARGO) fmt --check
	$(CARGO) clippy --all-targets -- -D warnings
	$(CARGO) test

.PHONY: all clean fclean re test
