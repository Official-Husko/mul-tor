# Import Standard Libraries
import os
import platform
from datetime import datetime

# Import Third-Party Libraries
import psutil
from termcolor import colored

# Import Local Libraries
from .storage import __version__ as version
from .pretty_print import PrettyPrint

class Logger:

    def __init__(self, message: str=None, extra: str=None, severity: str=None, site: str=None, size_limit: str=None) -> None:
        # File and Error Specific
        self.file_name = f"{datetime.today().strftime('%Y-%m-%d')}-runtime.log"
        self.message = message
        self.extra_info = extra
        self.severity = severity
        self.site = site
        self.exception_str: str = None
        self.size_limit = size_limit

        # OS Specific for logging
        self.os_bit: str = platform.machine()
        self.os_name: str = platform.system()
        self.device_name: str = platform.node()
        self.os_version: str = platform.version()
        self.os_cpu_name: str = platform.processor()
        self.os_cpu_cores: int = psutil.cpu_count(logical=False)
        self.tool_version: str = version
        self.os_ram: float = self._get_ram()
        self.os_release: str = platform.release()
        self.log_date: str = datetime.utcnow().strftime("%a %d-%b %H:%M:%S UTC %Y")
        self.os_family: str = os.name
        self.unix_time: float = datetime.utcnow().timestamp()

        self.pretty_print_instance = PrettyPrint()

        self._create_log_file()
        self._log_event()

    def _get_ram(self) -> float:
        # Get information about virtual memory
        virtual_mem = psutil.virtual_memory()

        # Get the total RAM in bytes
        total_ram = virtual_mem.total

        # You can convert it to Gigabytes (GB) for better readability
        return total_ram / (1024 ** 3)

    def _create_log_file(self) -> None:
        """
        Creates the log file if it doesn't exist. Adds the OS information to the log file for additional help when reporting issues.
        """
        if not os.path.exists(f"logs/{self.file_name}"):
            with open(self.file_name, "w", encoding="utf-8") as log_file:
                log_file.write(
                f"""
                OS: {self.os_bit} | {self.device_name} {self.log_date} | {self.os_name} ({self.os_family})
                CPU: {self.os_cpu_name} cores: {self.os_cpu_cores}
                RAM: {self.os_ram} GB
                Tool: {self.tool_version}
                Date: {self.unix_time}
                """
            )

    # TODO: Add more error handling like debug stuff
    def _log_event(self) -> None:
        """
        Logs the event based on the severity of the error.
        """

        # Handle most common errors
        if self.severity == "error":

            # If the site is Transfer_sh or Keep then print a different message because they need to fix it
            if self.site in ["Transfer.sh", "Keep"]:
                print(f"{self.pretty_print_instance.error} {colored(self.site, 'yellow')} fucked up again while uploading the file {colored(self.file_name, 'light_blue')}. Don't Report this! Its a known issue they need to fix.")
            else:
                print(f"{self.pretty_print_instance.error} An error occurred while uploading the file {colored(self.file_name, 'light_blue')} to {colored(self.site, 'yellow')}! Please report this. Exception: {colored(self.extra_info, 'red')}")
                self.exception_str = f"An error occurred while uploading the file {self.file_name} to {self.site}! Please report this. Exception: {self.extra_info}"
        
        elif self.severity == "size_error":
            print(f"{self.pretty_print_instance.error} File size of {colored(self.file_name, 'light_blue')} to big for {colored(self.site, 'yellow')}! Compress it to fit the max size of {colored(self.size_limit, 'yellow')}")
            self.exception_str = f"File size of {self.file_name} to big for {self.site}! Compress it to fit the max size of {self.size_limit}"

        elif self.severity == "debug":
            print(f"{self.pretty_print_instance.debug} {colored(self.message, 'cyan')}")

        else:
            print(f"{self.pretty_print_instance.error} An unknown error occured while uploading the file {colored(self.file_name, 'light_blue')} to {colored(self.site, 'yellow')}! Please report this. Exception: {colored(self.extra_info, 'red')}")
            self.exception_str = f"An unknown error occured while uploading the file {self.file_name} to {self.site}! Please report this. Exception: {self.extra_info}"

        with open(self.file_name, "a", encoding="utf-8") as log_dumper:
            log_dumper.writelines(f"{datetime.now()} | {self.message} | {self.exception_str}\n")
