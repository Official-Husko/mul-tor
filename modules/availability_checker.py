import requests
from termcolor import colored
from time import sleep

from .site_data import sites_data_dict
from .logger import Logger
from .pretty_print import *

available_sites = []

ping_sites = []

class Availability_Checker:
    
    def Evaluate(config):
        for site in sites_data_dict:
            if sites_data_dict[site]["apiKey"] == False and not site in config["blacklist"]:
                ping_sites.append(site)
            elif sites_data_dict[site]["apiKey"] == True and config["api_keys"][site]["apiKey"] != "" and not site in config["blacklist"]:
                ping_sites.append(site)
            else:
                pass
        
        for site in ping_sites:
            try:
                url = sites_data_dict[site]["api_url"]
                ping = requests.get(url)
                
                if ping.status_code == 200:
                    available_sites.append(site)
                else:
                    # Construct and save low level error
                    error_str = f"Site ping for {site} Failed! Error Code {ping.status_code}"
                    Logger.log_event(error_str, extra=str(ping))
            except Exception as e:
                # Construct and print the error
                error_str = f"An error occured while checking the sites! Please report this. Exception: {e}"
                print(colored(f"{error} {error_str}"))
                Logger.log_event(error_str, extra=str(ping))
                sleep(5)
                
        if available_sites == []:
                # Construct and print the error
                error_str = f"Available sites is empty. This should not be happening! Please report this."
                print(colored(f"{error} {error_str}"))
                Logger.log_event(error_str)
                sleep(5)
        
        return available_sites