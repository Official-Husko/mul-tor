import json
from termcolor import colored
from time import sleep

from .pretty_print import *
from .logger import Logger

current_config_version = "1.3.0"

class Config_Manager:
    
    def Checker():
        if os.path.exists("config.json"):
            config = Config_Manager.Reader()

            config_version = config.get("version", "0.0.0")

            if config_version < current_config_version:
                try:
                    os.remove("config_old.json")
                except:
                    pass
                os.rename("config.json", "config_old.json")
                Config_Manager.Writer()
            sleep(5)
        else:
            Config_Manager.Writer()

    def Reader():
        with open("config.json", "r") as cfg_file:
            config = json.load(cfg_file)
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
                    "autoLoadPreset": None,
                    "enablePresetSelection": False
                },
                "api_keys": {
                    "BowFile": {
                        "apiKey1": "",
                        "apiKey2": ""
                    }
                },
                "blacklist": ["SomeSiteName", "CheapGoFileCopy", "HotSinglesInYourArea"]
            }
            with open("config.json", "w") as cfg_file:
                json.dump(template, cfg_file, indent=6)
            print(colored(f"{info} New config file generated! Configure it and restart the program or wait 5 seconds and the program will continue with the default values"))
            print("")
            sleep(5)
            return template
        except Exception as e:
            # Construct and print the error
            error_str = f"An error occured during the writing of the config file! Please report this. Exception: {e}"
            print(colored(f"{error} {error_str}"))
            Logger.log_event(error_str)
            sleep(5)
