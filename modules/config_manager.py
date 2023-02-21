import json
from termcolor import colored
from time import sleep

from .pretty_print import *
from .logger import Logger

class Config_Manager:
    
    def Reader():
        with open("config.json", "r") as cfg_file:
            config = json.load(cfg_file)
        return config
    
    def Writer():
        try:
            template = {
                "checkForUpdates": True,
                "useProxies": False,
                "saveLinksToFile": True,
                "randomUserAgent": True,
                "api_keys": {
                    "example": {
                        "apiKey": "",
                        "email": ""
                    }
                },
                "blacklist": ["SomeSiteName", "CheapGoFileCopy", "HotSinglesInYourArea"]
            }
            with open("config.json", "w") as cfg_file:
                json.dump(template, cfg_file, indent=6)
            cfg_file.close()
            print(colored(f"{info} New config file generated! Configure it and restart the program. The program will continue with the default values"))
            print("")
            sleep(3)
        except Exception as e:
            # Construct and print the error
            error_str = f"An error occured during the writing of the config file! Please report this. Exception: {e}"
            print(colored(f"{error} {error_str}"))
            Logger.log_event(error_str)
            sleep(5)
        
    
    def Validate():
        # TODO: Validate Config
        pass