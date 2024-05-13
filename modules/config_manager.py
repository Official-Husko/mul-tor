# Import Standard Libraries
import json
from time import sleep
import os

# Import Third-Party Libraries
from termcolor import colored

# Import Local Libraries
from .pretty_print import info, error  # FIXME: Update to new pretty print code structure
from .logger import Logger

# TODO: probably should use jsonc instead of normal jsons: pip install jsonc-parser https://pypi.org/project/jsonc-parser/

class ConfigManager:
    """
    A class to manage configuration settings for an application.

    Attributes:
        filename (str): The name of the configuration file.
        latest_config_version (float): The latest version of the configuration.
        default_config (dict): The default configuration settings.
        config (dict): The current configuration settings.
    """

    def __init__(self):
        """
        Initializes the ConfigManager object.
        """
        self.filename = "app_config.json"
        self.latest_config_version = 2.0
        self.default_config = self._default_config()
        self.config = self._load_config()

    def _load_config(self):
        """
        Loads the configuration from the file, handles versioning, and migration if needed.

        Returns:
            dict: The loaded or default configuration.
        """
        if os.path.exists(self.filename):
            with open(self.filename, "r", encoding="utf-8") as cfg_file:
                config = json.load(cfg_file)

            config_version = config.get("version", "0.0.0")

            if config_version < self.latest_config_version:
                print(colored(f"{info} You are using an outdated config version! Trying to migrate config to new version.", "green"))
                try:
                    self._migrate_config()
                except Exception as e:
                    error_str = f"An error occurred during the migration of the config! Please report this. Exception: {e}"
                    print(colored(f"{error} {error_str}", 'red'))
                    Logger.log_event(error_str)
                    sleep(5)
                    print(colored(f"{info} Old one will be backed up and new one will be created.", "red"))
                    config = self._create_default_config()
            else:
                return config
        else:
            self._save_config()
            print(colored(f"{info} New config file generated! Configure it and restart the program or wait 5 seconds and the program will continue with the default values."), "green")
            print("")
            sleep(5)
            return self._default_config()

    def _default_config(self):
        """
        Returns the default configuration settings.

        Returns:
            dict: The default configuration.
        """
        return {
                "configHelpPage": "https://github.com/Official-Husko/mul-tor/wiki/Configuration",
                "version": self.latest_config_version,
                "checkForUpdates": True,
                "useProxies": False,
                "saveLinksToFile": True,
                "presetSystem": {
                    "enablePresetSelection": False,
                    "autoLoadPreset": False,
                    "presetName": "preset.json"
                },
                "api_keys": {
                    "BowFile": {
                        "apiKey1": "",
                        "apiKey2": ""
                    },
                    "DooDrive": {
                        "apiKey": "",
                        "apiToken": ""
                    },
                    "Mixdrop": {
                        "email": "",
                        "apiKey": ""
                    },
                    "Rapidgator": {
                        "email": "",
                        "password": ""
                    }
                },
                "blacklist": [
                    "SomeSiteName", 
                    "CheapGoFileCopy", 
                    "HotSinglesInYourArea"
                ],
                "proxySources": [
                    "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/http.txt",
                    "https://raw.githubusercontent.com/ShiftyTR/Proxy-List/master/http.txt",
                    "https://raw.githubusercontent.com/mmpx12/proxy-list/master/http.txt",
                    "https://raw.githubusercontent.com/Volodichev/proxy-list/main/http.txt",
                    "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt",
                    "https://raw.githubusercontent.com/roma8ok/proxy-list/main/proxy-list-http.txt"
                ]
            }
    
    def _save_config(self):
        """
        Saves the default configuration to the file.

        Returns:
            str: Status message indicating success or failure.
        """
        try:
            with open(self.filename, "w", encoding="utf-8") as cfg_file:
                json.dump(self._default_config(), cfg_file, indent=4)
            return "OK"
        except Exception as e:
            return f"An error occurred during the writing of the config file! Please report this. Exception: {e}"

    def _migrate_config(self):
        """
        Migrates the config to the latest version by adding any missing keys.

        Returns:
            dict: The migrated configuration.
        """
        for key, value in self.default_config.items():
            if key not in self.config:
                self.config[key] = value

        self._save_config()
        return self.config
