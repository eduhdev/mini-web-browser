import sys

from .network import DEFAULT_FILE, URL, extract_text, HTMLParser


def main():
    if len(sys.argv) > 1:
        url_text = sys.argv[1]
    else:
        url_text = "file://" + str(DEFAULT_FILE)

    url = URL(url_text)
    body = url.request()
    if url.view_source:
        print(body, end="")
    else:
        print(extract_text(HTMLParser(body).parse()), end="")
    print()


if __name__ == "__main__":
    main()
