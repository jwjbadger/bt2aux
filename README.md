# BT2AUX

This project acts as a bluetooth to aux convertor intended for automotive usage in cars that do not support Bluetooth to offer a safer way to manage audio streaming via tactile buttons that can be operated without looking as compared to viewing and using a phone while driving.

A circuit diagram is in the process of being made along with a PCB to make this project more universally applicable though its relatively simple electronic setup means it should be relatively easily reproduced from the following pictures:

![bt2aux](https://github.com/user-attachments/assets/2894b38d-59f5-4f0d-9930-33b8ada5eb0d)
![bt2aux_pcb](https://github.com/user-attachments/assets/6fc71446-a164-465d-b948-642789db0b4d)

The controller with tactile buttons is currently soldered on a seperate protoboard, which is attached to the original board via spare CAT6 cable though it can be easily reproduced with any type of cable and any four buttons. No electronic debouncing is needed as this is handled in code.

The default device name is "MY CAR" though it can be easily changed by searching for this keyword in the main.rs file, replacing it with any name of your choosing, and re-compiling.
