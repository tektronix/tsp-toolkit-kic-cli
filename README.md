# Keithley Instrument Communicator

The Keithley Instrument Communicator (KIC) is a command-line application that enables simple communication to an instrument. 

## Certificates Setup

Tektronix uses self-signed certificates. This makes life difficult when using standard tooling like `npm`. In order to use the GitLab npm package registry, you'll need to tell npm that those self-signed certificates are safe. `npm` annoyingly doesn't use your system's installed CA certs, so we have to add to the accepted ones.

1. Download the [ca-certs](https://git.keithley.com/devops/certificates/-/jobs/artifacts/main/download?job=package) from the devops/certificates project (link will start download).
2. Extract and put them somewhere (I personally like `~/.certs/Tektronix_*.crt`)
3. Add NODE_EXTRA_CA_CERTS to your environment variables with the full path to the `Tektronix_Global.gitlab.chain1.crt` file

## Getting started
<details>
<summary>Developer Setup</summary>

## Developer Setup

If you are developing on this project, it is in your best interest to add the provided pre-commit hook to your local `.git/hooks` folder.

```bash
cp scripts/pre-commit .git/hooks/pre-commit
```

This will make commits take longer, but will ensure CI/CD doesn't fail due to linting, formatting, or build errors that should have been caught before your commit.

If there is a specific reason you want to finish the commit without running the pre-commit checks (e.g. You want to commit a work in progress that doesn't build, or there is a specific reason to commit to `dev`), you can run the following in a terminal:

```bash
git commit -m "<COMMIT_MESSAGE>" --no-verify
```

</details>

## Installation

### Method 1: `npm`
Make sure you have [set up the certificates](#certificates-setup) and have [set up a Personal Access Token](https://git.keithley.com/-/profile/personal_access_tokens?name=NPM+Access+Token&scopes=api,read_user,read_registry,read_repository) (keep track of that token, you'll need to generate a new one if you lose it).

This can be installed by running 

```bash
$ npm config set @trebuchet:registry https://git.keithley.com/api/v4/projects/33/packages/npm
$ npm config set -- '//git.keithley.com/api/v4/projects/33/packages/npm:_authToken' "<your_token>"
$ npm install -g @trebuchet/ki-comms
```

You can update to the latest release using

```bash
$ npm install -g @trebuchet/ki-comms
```

### Method 2: Download the Executable

You can find the latest releases [here](https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/releases/permalink/latest). Be sure to pick the correct one for your OS.

### Method 3: Use it in the [teaspoon-comms](https://git.keithley.com/trebuchet/teaspoon/teaspoon-comms) extension

You can find the latest releases [here](https://git.keithley.com/trebuchet/teaspoon/teaspoon-comms/-/releases/permalink/latest)

## Usage

If you used the npm installation method, you can find all the usage information by running

```bash
# on Windows
$ windows-kic --help

# on Linux
$ linux-kic --help
```

If you downloaded the executable (add a `.exe` if on Windows):
```bash
# if on your PATH
kic --help

# if in local directory
./kic --help
```

## Support
If you find issues or have a feature request, please enter a [new issue on GitLab](https://git.keithley.com/trebuchet/teaspoon/ki-comms/-/issues/new). 
This will allow us to filter the issues into JIRA to avoid duplicates and keep things focussed.

## Roadmap

- [x] LAN connection to TSP-enabled Keithley instruments
- [x] Discovery of TSP-enabled Keithley instruments on LAN
- [x] Send script files to TSP-enabled Keithley instruments on LAN
- [x] Log into password-protected instruments on TSP-enabled Keithley instruments over LAN
- [ ] USB connection to TSP-enabled Keithley instruments
- [ ] Discovery of TSP-enabled Keithley instruments on USB
- [ ] Send script files to TSP-enabled Keithley instruments on USB
- [ ] Log into password-protected instruments on TSP-enabled Keithley instruments over USB


## Contributing
Please see [Contributing.md](./CONTRIBUTING.md).

## Authors and acknowledgment
- [Edwin Sarver](https://git.keithley.com/edwin.sarver)
- [Karthik E](https://git.keithley.com/Karthik.E)
- [Pavan Narayana](https://git.keithley.com/pavan.narayana)
- [Prithviraj V](https://git.keithley.com/prithviraj.v)
- [Rajeev Jha](https://git.keithley.com/rajeev.jha)
- [Shreya Gt](https://git.keithley.com/shreya.gt)
- [Sydney Tenaglia](https://git.keithley.com/sydney.tenaglia)
- [Syed Jaffery](https://git.keithley.com/syed.jaffery)

## License
TBD

## Project status
Alpha!
