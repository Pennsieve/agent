.PHONY: all docs
.DEFAULT_GOAL := all

all: docs

clean:
	rm -Rf _site/

docs:
	bundle install >/dev/null 2>&1
	bundle exec jekyll build
