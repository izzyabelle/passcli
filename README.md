# passcli

A command-line password manager written in Rust.

# Usage

  `passcli [OP] [ACCOUNT] [FIELD] [OPTIONS]`
  
  OPs:
    a - add
    p - print
    e - edit
    r - remove

  All fields are optional, any missing required argument triggers a prompt or
  uses a default, such as 'pass' for field. Some are configurable.

  All options listed with `passcli --help`

  `passcli` defaults to `passcli p --all-fields`

## Examples

  `passcli a codeberg`
    - Prompts for the 'pass' field for codeberg.

  `pasccli a firefox email -v izzy@belle.co.uk`
    - Adds the email field promptless.

  `passcli a Companies\ House -g 32 --disallow symbol`
    - Generates a 32 character random 'pass' with no symbols

  `passcli p Companies\ House --quiet | *$your_os_clipboard_copy*`
    - Copy the random password next year to your clipboard

  ```
     passcli a secret_data dev -v /dev/sda1 &&                        \
  && passcli a secret_data -g --quiet                                 \
  |  cryptsetup luksFormat $(passcli p secret_data dev) secret-volume
  ```
    - Encrypt your secret data with the default passgen lenth of 16
  
