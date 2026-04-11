## Atrium Browser CLI

import os, strutils, sequtils

type
  Command = enum
    cmdOpen
    cmdNewWindow
    cmdInstall
    cmdProfile
    cmdHelp
    cmdVersion

  CliConfig = object
    command: Command
    url: string
    profile: string
    newWindow: bool

proc printHelp() =
  echo """
Atrium Browser v0.1.0

Usage:
  atrium [URL]           Open a URL
  atrium [OPTIONS]

Options:
  -h, --help             Show this help message
  -v, --version          Show version information
  --new-window           Open in a new window
  --profile NAME         Use specified profile
  install EXTENSION      Install .yz extension

Examples:
  atrium https://example.com
  atrium --new-window https://example.com
  atrium --profile work
  atrium install extension.yz
"""

proc parseArgs(args: seq[string]): CliConfig =
  result = CliConfig(
    command: cmdOpen,
    url: "",
    profile: "default",
    newWindow: false
  )

  var i = 1  # Skip program name
  while i < args.len:
    let arg = args[i]
    
    if arg == "--help" or arg == "-h":
      result.command = cmdHelp
      return
    elif arg == "--version" or arg == "-v":
      result.command = cmdVersion
      return
    elif arg == "--new-window":
      result.newWindow = true
    elif arg == "--profile":
      if i + 1 < args.len:
        inc i
        result.profile = args[i]
        result.command = cmdProfile
    elif arg == "install":
      result.command = cmdInstall
      if i + 1 < args.len:
        inc i
        result.url = args[i]
    elif arg.startsWith("-"):
      stderr.write("Unknown option: ", arg, "\n")
      printHelp()
      quit(1)
    else:
      result.url = arg
      result.command = cmdOpen
    
    inc i

proc main() =
  let args = commandLineParams()
  
  if args.len == 0:
    printHelp()
    quit(0)
  
  let config = parseArgs(args)
  
  case config.command
  of cmdHelp:
    printHelp()
  
  of cmdVersion:
    echo "Atrium Browser v0.1.0"
    echo "Built with Rust, Odin, Wren, Nim"
  
  of cmdOpen:
    if config.url == "":
      echo "No URL specified"
      printHelp()
      quit(1)
    
    echo "Opening: ", config.url
    if config.newWindow:
      echo "In new window"
    if config.profile != "default":
      echo "Using profile: ", config.profile
    
    # In production: launch browser with URL
    echo "Launching browser..."
  
  of cmdNewWindow:
    echo "Opening new window"
  
  of cmdInstall:
    if config.url == "":
      echo "No extension file specified"
      quit(1)
    
    echo "Installing extension: ", config.url
    
    # In production: parse .yz file and install
    if not fileExists(config.url):
      stderr.write("File not found: ", config.url, "\n")
      quit(1)
    
    echo "Extension installed successfully!"
  
  of cmdProfile:
    echo "Using profile: ", config.profile
    echo "Profile loaded"

when isMainModule:
  main()
