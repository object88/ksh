# ksh

Run a shell in a kubernetes environment with local files.

## ksh run

The `ksh run` command will spin up a new pod

```sh
ksh run --profile foo
```

### flags

| flag | type | value | description
| --- | --- | --- | --- |
| namespace | string | the current namespace | the kubernetes namespace to run the pod within |
| retain | bool | false | whether to keep the pod after exiting the shell; note that if set, instead of the interactive shell being pid 1, it will be `sh` running `sleep infinite` |




## ksh exec

## ksh debug

## Common flags

These flags effect all of the above commands

| flag | type | value | description
| --- | --- | --- | --- |
| config | path | the `ksh` subfolder under the user config directory (following XDG standards) | 
| context | string | the current kubeconfig context | the kubernetes context to run whtin |
