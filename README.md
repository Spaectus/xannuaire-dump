# XAnnuaire dump

Allows you to retrieve the entire contents of the [XAnnuaire](https://extranet.polytechnique.fr/xannuaire/login/index.php) in a csv file. You must have login credentials to access the XAnnuaire.

## Requirement

You need cargo/rust installed.

## Manual Installation

```
cargo build --release
```

## Usage

You must give your login to the XAnnuaire via two environment variables :
- XANNUAIRE_USERNAME
- XANNUAIRE_PASSWORD

See usage :
```
target/release/xannuaire-dump.exe --help
```

There are two modes of data retrieval, the "brief" and the normal. Refer to the table below to compare the data retrieved by these two modes:

|              	| Retrieved in normal mode ? 	| Retrieved in brief mode ?                                                  	|
|--------------	|----------------------------	|----------------------------------------------------------------------------	|
| uid          	| Yes                        	| Yes                                                                        	|
| name         	| Yes                        	| Yes                                                                        	|
| rattach      	| Yes                        	| Yes                                                                        	|
| rattach_full 	| Yes                        	| Yes                                                                        	|
| phone_number 	| Yes when available         	| Yes when available                                                         	|
| email        	| Yes                        	| No but can be deduced from the uid :<br>email = uid + "@polytechnique.edu" 	|
| desk         	| Yes when available         	| No                                                                         	|
| image_xid    	| Yes                        	| No                                                                         	|
| image_base64 	| Yes when available         	| No                                                                         	|

### Examples :

Retrieve all data (normal mode)  :

```
target/release/xannuaire-dump.exe xannuaire.csv
```

Brief mode :

```
target/release/xannuaire-dump.exe --brief xannuaire.csv
```

