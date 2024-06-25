# Import Standard Libraries
from time import sleep
import random
import json

# Import Third-Party Libraries
import requests
from termcolor import colored

# Import Local Libraries
from .site_data import sites_data_dict
from .logger import Logger
from .pretty_print import PrettyPrint
from .storage import __dev_debug__ as DEBUG
from .storage import __user_agent__ as USER_AGENT



class AvailabilityChecker:

    def __init__(self, config, proxy_list):
        self.config = json.loads(config)
        self.proxy_list = proxy_list
        self.available_sites: list[str] = []
        self.ping_sites: list[str] = []

        self.pretty_print_instance = PrettyPrint()
        
    def _check_skip_site_config(self) -> bool:
        return self.config.get("skip_site_check", False)

    def _blacklist_check(self, site: str):
        # return bool if site is in available sites. this is to filter out blacklisted sites.
        return site in self.available_sites

    def _check(self):


        blacklist = []
        for blacklisted_site in self.config.get("blacklist", []):
            blacklist.append(blacklisted_site.lower())
        for site in sites_data_dict:
            if DEBUG:
                print(f"{colored('Checking:', 'green')} {site}")
            if not site.lower() in blacklist:
                self.ping_sites.append(site)
            else:
                pass
        print(f"{colored('Checking', 'green')} {colored(len(self.ping_sites), 'yellow')} {colored('supported sites...', 'green')}", end='\r')

        for site in self.ping_sites:
            try:
                url = sites_data_dict[site]["api_url"]
                proxies = random.choice(self.proxy_list) if self.proxy_list else None
                
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
