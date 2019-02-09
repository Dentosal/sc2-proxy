# SC2-Proxy, StarCraft II bot API management layer

## Features
* Starts one or more SC2 processes
    * Manages port configurations
    * Abstracts away game hosting
* Minimal overhead
    * Should be suitable for rendered interface as well

## Future Goals
* SC2 process pooling
    * Reuse processes
    * Prelanuch on startup?
* Resource management and limits
    * Time used by each participant
    * Number of API calls
    * APM limit
    * Debug / cheat commands
* Metrics, e.g. timing, overhead and request counts
* Automated test suite
* Command line interface
    * Machine readable output mode
* Remote control endpooint
    * JSON over TCP
    * Dynamic configuration
    * Off-band requests
    * Live statistics

## Non-goals
* Full API abstraction layer
* Automatic action bundling
* Query result caching, especially during single step
* Action result state tracking
