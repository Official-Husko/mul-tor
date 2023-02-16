import requests
import json
from datetime import datetime, timedelta, date
from termcolor import colored
from .pretty_print import *
from .logger import *
import os
from time import sleep

user_agents_list = []

backup_list = [
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36 Edg/109.0.1518.70",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:109.0) Gecko/20100101 Firefox/109.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.2 Safari/605.1.15",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/16.3 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.67 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:102.0) Gecko/20100101 Firefox/102.0",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:88.0) Gecko/20100101 Firefox/88.0",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/109.0.0.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/101.0.4951.54 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/104.0.0.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/100.0.4896.75 Safari/537.36 Edg/100.0.1185.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/105.0.0.0 Safari/537.36 Edg/105.0.1343.42"
]

class UserAgentManager:
    
    def Scraper():
        if os.path.exists("user_agents.json"):
            os.remove("user_agents.json")
        try:
            req = requests.get("https://www.useragents.me/api").json()
            user_agents_list = []
            for ua in req["data"]:
                user_agents_list.append(ua["ua"])
            with open("user_agents.json", "w") as writer:
                template = {
                    "creation_date": f"{date.today()}",
                    "user_agents": user_agents_list
                }
                json.dump(template, writer, indent = 6)
            return user_agents_list
        except Exception as e:
            
            # Construct and print the error
            error_str = f"An error occured during the Fetching of user_agents! Please report this. Exception: {e}"
            print(colored(f"{warning} {error_str}"))
            print(colored(f"{warning} Defaulting back to backup user-agents"))
            Logger.log_event(error_str, req)
            
            # Delete the corrupted json file
            if os.path.exists("user_agents.json"):
                os.remove("user_agents.json")
                
            # Write a backup file to use
            with open("user_agents.json", "w") as writer:
                template = {
                    "creation_date": f"1970-01-01",
                    "user_agents": backup_list
                }
                json.dump(template, writer, indent = 6)
            
            sleep(5)
            return user_agents_list
            
    def Reader():
        with open("user_agents.json", "r") as ua:
            ua_list = json.load(ua)
            status = UserAgentManager.Verify(ua_list["creation_date"])
            ua.close()
            if status == "OUTDATED":
                UserAgentManager.Scraper()
            else:
                return ua_list["user_agents"]
            
    def Verify(fldate):
        file_date = datetime.strptime(fldate, "%Y-%m-%d")
        today_date = date.today()
        max_date = file_date + timedelta(days=10)
        
        if str(today_date) > str(max_date):
            return "OUTDATED"
        else:
            return "OK"
        