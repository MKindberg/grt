# grt

A cli for searching Gerrit and use fuzzy finding to select a commit to checkout or cherry-pick.

## Usage

`grt [options] command search-query`

Valid commands are `checkout` or `co` for checking out the chosen commit and `cherry-pick` or `cp` for cherry-picking.

Options:
|Short|long|Description|
| -- | -------| --------|
| -h | --help |     Print help menu|
| -u URL | --url URL  |     The url to Gerrit, can also be set with en env var GERRIT_URL or guessed|
| -p NAME | --project NAME|  The project to search in (will check remote.origin.projectname by default)|
| -c | --closed     |   Include closed commits|
| -o | --open       |   Don't include closed commits (default, will override -c if set)|
| -d | --download-method   |  Method used to download the commit, valid options are 'ssh' (default), 'http' and 'anon' (anonymous http)  |
|   | --debug      |   Print debug information while running|
|-f FILE | --file FILE  |   Read json data from file instead of Gerrit|

The options can be set either on command line or through
the env var GRT_ARGS, anything set on command line will
override what's set in the environment.
