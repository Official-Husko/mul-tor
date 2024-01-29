import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from .logger import Logger

from main import DEBUG

site = "FileMail"

"""
Last Checked 26/01/2024
"""

"""

Shitty 2 uploads limit per day. Even the initializing counts as one.

"""

class FileMail:
    
    def Uploader(file, proxy_list, user_agents, api_keys):
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            base_url = sites_data_dict[site]["download_url_base"]
            server_url = sites_data_dict[site]["server_url"].format(api_key=api_key)
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '') if len(file_name) > 240 else file_name # Changed from 255 to 15 as an additional safety net.

            # upload_url = sites_data_dict[site]["url"].format(server=server)
                
            headers = {"User-Agent": ua, "Content-Type": "application/octet-stream", "accept": "application/json"}

            # Initialize file upload and get server url
            if proxy_list == []:
                req = requests.post(url=server_url, headers=headers)
            else:
                req = requests.put(url=server_url, data=file_upload, headers=headers, proxies=random.choice(proxy_list))


            with open(file, "rb") as file_upload:
                if proxy_list == []:
                    req = requests.post(url=rand_url, data=file_upload, headers=headers)
                else:
                    req = requests.put(url=rand_url, data=file_upload, headers=headers, proxies=random.choice(proxy_list))
                file_upload.close()
            if req.status_code == 201:
                return {"status": "ok", "file_name": file_name, "file_url": rand_url}
            else:
                raise Exception("Wrong Response Code. View request body below or in log file.")
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}