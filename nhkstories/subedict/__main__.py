#!/usr/bin/env python
import argparse

import jmdict


def main():
    # parse arguments
    parser = argparse.ArgumentParser()
    parser.description = 'Reduce EDICT file to cover given text'
    parser.add_argument('edict_file')
    parser.add_argument('text_file')
    args = parser.parse_args()

    # load EDICT file
    dictionary = jmdict.load_from_filename(args.edict_file)

    # read text file
    with open(args.text_file) as f:
        text = f.read()

    # actually do the work
    sub_dictionary = jmdict.subedict(dictionary, text)

    # sort and output
    output = ''.join(sorted(sub_dictionary))
    print(output, end='')


if __name__ == '__main__':
    main()
