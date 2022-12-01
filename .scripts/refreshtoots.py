#!/usr/bin/env python
"""
Script to create Hugo markdown
files from Mastodon Toots
"""

from urllib import request
from glob import glob
from http.client import HTTPResponse
from pathlib import Path
from typing import Any, Dict, Optional
import json
import math
import sys

TOOT_CONTENT_LOCATION = "./"
SERVER="https://fosstodon.org"
# Quick way to find user id: https://prouser123.me/mastodon-userid-lookup/
MUID=108219415927856966
# Server default (when < 0) is 20
RETRIEVE_NUM_TOOTS=1000
MAX_TOOTS_PER_QUERY=40 # Cannot change (server default)
MAX_TOOT_ID=-1


def retrieve_toots_from_server():
    """
    Grabs toots from Mastodon server
    """
    global MAX_TOOT_ID
    server_data = []

    for _ in range(math.ceil(RETRIEVE_NUM_TOOTS // MAX_TOOTS_PER_QUERY)):
        # Grab toots from Mastodon
        limit_param = "?limit=" + str(RETRIEVE_NUM_TOOTS) \
            if RETRIEVE_NUM_TOOTS > 0 else "?"
        max_id = "&max_id=" + str(MAX_TOOT_ID) \
            if MAX_TOOT_ID > 0 else ""
        url = SERVER + "/api/v1/accounts/" + str(MUID) + "/statuses" + limit_param + max_id
        response: Optional[HTTPResponse] = None

        try:
            response = request.urlopen(url)
        except Exception:
            print("Unable to grab toots from Mastodon.")

        if response is None:
            sys.exit(-1)

        # Parse server response
        server_data_part: Optional[list] = None
        try:
            server_data_part = json.loads(response.read())
        except Exception:
            print("Malformed JSON response from server.")

        if server_data is None:
            sys.exit(-1)

        if not isinstance(server_data_part, list):
            print("Unexpected JSON response, should be of form list.")
            sys.exit(-1)

        # No more to retrieve
        if len(server_data_part) == 0:
            break

        print(f"Retrieved {len(server_data_part)} toots from server")
        server_data.extend(server_data_part)
        MAX_TOOT_ID = int(min(server_data_part, key=lambda p: int(p['id']))['id'])

    print(f"Successfully grabbed a total of {len(server_data)} toots from server")
    return server_data


def findall(p, s):
    """
    Yields all the positions of
    the pattern p in the string s.
    Source: https://stackoverflow.com/a/34445090
    """
    i = s.find(p)
    while i != -1:
        yield i
        i = s.find(p, i+1)


def read_json_frontmatter(markdown_contents) -> Optional[Dict[Any, Any]]:
    """
    Take the contents from a Hugo markdown
    file and read the JSON frontmatter if it
    exists.
    """
    front_matter_indices = list(findall('---', markdown_contents))
    if len(front_matter_indices) < 2:
        return None
    front_matter = markdown_contents[(front_matter_indices[0] + 3):front_matter_indices[1]]
    front_matter_json = None
    try:
        front_matter_json = json.loads(front_matter)
    except Exception:
        pass
    if not isinstance(front_matter_json, dict):
        front_matter_json = None
    front_matter_json['content'] = markdown_contents[front_matter_indices[1] + 19:-17]
    return front_matter_json

def reformat_toot(toot_json):
    """
    Takes a toot_json and
    slightly modifies it to match
    some of the fields Hugo expects.
    """
    # Turn URL -> Syndication
    toot_url = toot_json['url']
    del toot_json['uri']
    del toot_json['url']
    toot_json['syndication'] = toot_url
    # Turn Created At -> Date
    toot_date = toot_json['created_at']
    del toot_json['created_at']
    toot_json['date'] = toot_date
    # Strip out highly dynamic account information
    del toot_json['account']['locked']
    del toot_json['account']['bot']
    del toot_json['account']['discoverable']
    del toot_json['account']['group']
    del toot_json['account']['created_at']
    del toot_json['account']['note']
    del toot_json['account']['followers_count']
    del toot_json['account']['following_count']
    del toot_json['account']['statuses_count']
    del toot_json['account']['last_status_at']
    del toot_json['account']['emojis']
    del toot_json['account']['fields']


def create_toot(toot_json):
    """
    Takes a JSON toot from Mastodon
    and creates a string representing
    the contents of a Hugo markdown
    file.
    """
    toot_content = toot_json['content']
    del toot_json['content']
    return "---\n" + \
        f"{json.dumps(toot_json)}\n" +\
        "---\n" +\
        "{{< unsafe >}}\n" +\
        f"{toot_content}\n" +\
        "{{< /unsafe >}}\n"

def toot_file_from_id(tootid):
    """Returns toot filename from id"""
    return f"{TOOT_CONTENT_LOCATION}/{tootid}.md"

def read_toot(tootid) -> Optional[Dict[Any, Any]]:
    """
    Given a toot id, return
    the markdown file contents
    of the toot stored in Hugo
    if it exists.
    """
    try:
        with open(toot_file_from_id(tootid), "r", encoding="UTF-8") as toot_file:
            toot_data = read_json_frontmatter(toot_file.read())
            return toot_data
    except Exception:
        return None

def write_toot(toot):
    """
    Takes a toot json
    and writes it to a hugo
    markdown content file.
    """
    toot_id = toot['id']
    try:
        with open(toot_file_from_id(toot_id), "w", encoding="UTF-8") as toot_file:
            toot_file.write(create_toot(toot))
    except Exception as e:
        print("Failed to write toot", toot_id)

# Read in saved toot data
toot_filenames = glob(TOOT_CONTENT_LOCATION + "/*.md")
toot_ids = { Path(fname).stem for fname in toot_filenames }

server_toots = retrieve_toots_from_server()

for stoot in server_toots:
    # Skip boosts for now
    if stoot['content'] == '':
        continue

    reformat_toot(stoot)
    stoot_id = stoot['id']


    # If the toot already exists
    if stoot_id in toot_ids:
        saved_tootdata = read_toot(stoot_id)
        if saved_tootdata is None:
            print("Unable to read saved toot id", stoot_id)

        # Only update if toot has changed
        elif saved_tootdata != stoot:
            print("Updating toot id", stoot_id)
            write_toot(stoot)

    # New toot found
    else:
        print("Creating toot id", stoot_id)
        write_toot(stoot)

print("Completed")
