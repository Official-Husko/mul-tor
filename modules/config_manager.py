import json
from termcolor import colored
from time import sleep
import os

from .pretty_print import *
from .logger import Logger

current_config_version = "1.5.0"

class Config_Manager:
    
    def Checker():
        if os.path.exists("config.json"):
            with open("config.json", "r") as cfg_file:
                config = json.load(cfg_file)

            config_version = config.get("version", "0.0.0")
            advancedMode = config.get("advancedMode", False)

            if config_version < current_config_version and advancedMode == False:
                try:
                    os.remove("config_old.json")
                except:
                    pass
                os.rename("config.json", "config_old.json")
                print(colored(f"{info} You are using an outdated config version! Old one is backed up. Creating New one.", "green"))

                config = Config_Manager.Writer()
                return config
            else:
                return config

        else:
            config = Config_Manager.Writer()
            return config
    
    def Writer():
        try:
            template = {
                "version": current_config_version,
                "checkForUpdates": True,
                "useProxies": False,
                "saveLinksToFile": True,
                "randomUserAgent": False,
                "presetSystem": {
                    "autoLoadPreset": False,
                    "presetName": "",
                    "enablePresetSelection": False
                },
                "advancedMode": False,
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
                    },
                    "Pixeldrain": {
                        "ApiKey": "",
                    }
                },
                "blacklist": ["SomeSiteName", "CheapGoFileCopy", "HotSinglesInYourArea"]
            }
            with open("config.json", "w") as cfg_file:
                json.dump(template, cfg_file, indent=6)
            print(colored(f"{info} New config file generated! Configure it and restart the program or wait 5 seconds and the program will continue with the default values."), "green")
            print("")
            sleep(5)
            return template
        except Exception as e:
            # Construct and print the error
            error_str = f"An error occurred during the writing of the config file! Please report this. Exception: {e}"
            print(colored(f"{error} {error_str}"))
            Logger.log_event(error_str)
            sleep(5)
