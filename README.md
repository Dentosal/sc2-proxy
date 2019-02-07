# SC2-Proxy, StarCraft II bot API management layer

## Goals
* Starts one or more SC2 processes
    * Manages port configurations
    * Abstracts away game hosting
* Minimal overhead
    * Should be suitable for real mode rendered interface as well
* Resource management and limits
    * Time used by each participant
    * Number of API calls
    * APM limit
    * Debug / cheat commands
* Metrics, e.g. timing and overhead
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
