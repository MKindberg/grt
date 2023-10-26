# grt

A cli for searching Gerrit and use fuzzy finding to select a commit to checkout or cherry-pick.

## Usage

Just download the binary to somewhere in your path and run

`grt [options] <command> <search-query>`

Valid commands are `checkout` or `co` for checking out the chosen commit and `cherry-pick` or `cp` for cherry-picking.

Options:
|Short|long|Description|
| -- | -------| --------|
| -h | --help |     Print help menu |
| -u URL | --url URL  |     Override the auto-detected gerrit url |
| -c | --closed     |   Include closed commits|
| -o | --open       |   Don't include closed commits (default, will override -c if set)|
|   | --debug      |   Print debug information while running|
|-f FILE | --file FILE  |   Read json data from file instead of Gerrit|

The options can be set either on command line or through
the env var GRT_ARGS, anything set on command line will
override what's set in the environment.
