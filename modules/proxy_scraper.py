# Import Standard Libraries

# Import Third-Party Libraries
import requests
from termcolor import colored

# Import Local Libraries
from .storage import __version__ as version
from .storage import __user_agent__ as user_agent
from .pretty_print import PrettyPrint
from .logger import Logger


# scrape proxies from a given destination
class ProxyScraper():
    """Fetch a fresh list of proxies for usage during runtime.
    """
    
    def __init__(self, proxy_source_list: list=None) -> None:
        self.proxy_source_list = proxy_source_list
        self.proxy_list = self._scraper()
        self.user_agent = user_agent

    def _scraper(self):
        try:
            proxy_list = []
            for source in self.proxy_source_list:
                response = requests.get(
                    url=source,
                    headers={"User-Agent":self.user_agent},
                    timeout=30
                )
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



            logger_instance = Logger(message="An error occurred during the fetching of proxies!", extra=str(e), severity="error")

            error_str = f"An error occurred during the fetching of proxies! Please report this. Exception: {e}"
            print(colored(f"{error} {error_str}"))
            Logger.log_event(error_str, response)