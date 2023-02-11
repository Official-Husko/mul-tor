import requests
from main import version
from time import sleep
from termcolor import colored
from .pretty_print import *
from .logger import Logger

proxy_source_list = [
    "https://raw.githubusercontent.com/TheSpeedX/SOCKS-List/master/http.txt",
    "https://raw.githubusercontent.com/ShiftyTR/Proxy-List/master/http.txt",
    "https://raw.githubusercontent.com/mmpx12/proxy-list/master/http.txt",
    "https://raw.githubusercontent.com/Volodichev/proxy-list/main/http.txt",
    "https://raw.githubusercontent.com/monosans/proxy-list/main/proxies/http.txt",
    "https://raw.githubusercontent.com/roma8ok/proxy-list/main/proxy-list-http.txt"
]

# scrape proxies from a given destination
class ProxyScraper():
    """Fetch a fresh list of proxies for usage during runtime.
    """
    
    def Scraper():
        try:
            proxy_list = []
            for source in proxy_source_list:
                sleep(1)
                response = requests.get(source,headers={"User-Agent":f"mul-tor/{version} (by Official Husko on GitHub)"},timeout=10)
                proxy_raw = response.text
                split_proxies = proxy_raw.split()
                for proxy in split_proxies:
                    if proxy in proxy_list:
                        break
                    else:
                        proxyy = {"http": proxy}
                        proxy_list.append(proxyy)
            return proxy_list
        except Exception as e:
            # Construct and print the error
            error_str = f"An error occured during the fetching of proxies! Please report this. Exception: {e}"
            print(colored(f"{error} {error_str}"))
            Logger.log_event(error_str, response)