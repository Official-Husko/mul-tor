import requests
from termcolor import colored
from time import sleep
import random

from .site_data import sites_data_dict
from .logger import Logger
from .pretty_print import *

available_sites = []

ping_sites = []

class Availability_Checker:
    
    def Evaluate(config, proxy_list, ua_list):
        blacklist = []
        for blacklisted_site in config["blacklist"]:
            blacklist.append(blacklisted_site.lower())
        for site in sites_data_dict:
            # TODO: does not get api keys so fix shit shit cunt
            if sites_data_dict[site]["apiKey"] == False and not site.lower() in blacklist:
                ping_sites.append(site)
            elif sites_data_dict[site]["apiKey"] == True and config.get("api_keys", {}).get(site.lower(), {}).get("apiKey", "") != "" and not site.lower() in blacklist:
                ping_sites.append(site)
            else:
                pass
        print(colored("Checking available sites...", "green"), end='\r')
        
        for site in ping_sites:
            try:
                ua = random.choice(ua_list)
                url = sites_data_dict[site]["api_url"]
                
                if proxy_list == []:
                    ping = requests.get(url, headers={"User-Agent": ua}, timeout=5)
                else:
                    proxy = random.choice(proxy_list)
                    ping = requests.get(url, headers={"User-Agent": ua}, proxies=proxy, timeout=5)
                
                if ping.status_code == 200:
                    available_sites.append(site)
                else:
                    # Construct and save low level error
                    error_str = f"Site ping for {site} Failed! Error Code {ping.status_code}"
                    Logger.log_event(error_str, extra=str(ping))
            except (requests.exceptions.Timeout, requests.exceptions.ConnectionError) as e:
                # Construct the error
                error_str = f"An error occured while checking the sites! Please report this. Exception: {e}"
                Logger.log_event(error_str, extra=str(ping))
                sleep(5)
            except Exception as e:
                # Construct and print the error
                error_str = f"An error occured while checking the {site}! Please report this. Exception: {e}"
                print(colored(f"{error} {error_str}"))
                Logger.log_event(error_str, extra=str(ping))
                sleep(5)
                
        if available_sites == []:
                # Construct and print the error
                error_str = f"Available sites is empty. This should not be happening! Please report this."
                print(colored(f"{error} {error_str}"))
                Logger.log_event(error_str)
                sleep(5)
        
        print(f"{colored(len(available_sites), 'yellow')} {colored('Available Sites          ', 'green')}")
        
        print("")
        
        return available_sites