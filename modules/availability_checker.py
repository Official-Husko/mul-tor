from time import sleep
import random
import json
import urllib3
from termcolor import colored

from main import DEBUG, SKIP_SITE_CHECK, USER_AGENT
from .site_data import sites_data_dict
from .logger import Logger
from .pretty_print import error

available_sites = []
ping_sites = []


class AvailabilityChecker:

    def __init__(self, config, proxy_list):
        self.config = json.loads(config)
        self.proxy_list = proxy_list
        






    def Evaluate(config, proxy_list):
        blacklist = []
        for blacklisted_site in config.get("blacklist", []):
            blacklist.append(blacklisted_site.lower())
        for site in sites_data_dict:
            if DEBUG:
                print(f"{colored('Checking:', 'green')} {site}")
            if not site.lower() in blacklist:
                ping_sites.append(site)
            else:
                pass
        print(f"{colored('Checking', 'green')} {colored(len(ping_sites), 'yellow')} {colored('supported sites...', 'green')}", end='\r')
        
        for site in ping_sites:
            try:
                url = sites_data_dict[site]["api_url"]
                proxies = random.choice(proxy_list) if proxy_list else None
                
                if DEBUG and SKIP_SITE_CHECK:
                    available_sites.append(site)
                else:
                    ping = requests.get(url, headers={"User-Agent": USER_AGENT}, proxies=proxies, timeout=5)
                
                    if ping.status_code == 200:
                        available_sites.append(site)
                    else:
                        # Construct and save low level error
                        error_str = f"Site ping for {site} Failed! Error Code {ping.status_code}"
                        Logger.log_event(error_str, extra=str(ping))
            
            except (requests.exceptions.Timeout, requests.exceptions.ConnectionError) as e:
                # Construct the error
                error_str = f"An error occurred while checking {site}! Please report this. Exception: {e}"
                Logger.log_event(error_str, extra=str(ping))
                sleep(5)
            except Exception as e:
                # Construct and print the error
                error_str = f"An error occurred while checking {site}! Please report this. Exception: {e}"
                print(colored(f"{error} {error_str}"))
                Logger.log_event(error_str, extra=str(ping))
                sleep(5)
                
        if available_sites == []:
                # Construct and print the error
                error_str = f"Available sites is empty. This should not be happening! Please report this."
                print(colored(f"{error} {error_str}"))
                Logger.log_event(error_str)
                sleep(5)
        
        print(f"{colored(len(available_sites), 'yellow')} {colored('Available Sites                ', 'green')}")
        
        print("")

        return available_sites
