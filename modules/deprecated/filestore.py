import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "FileStore"

"""
Last Checked 24/03/2024
"""

"""

"FileStore": {
    "apiKey": True,
    "url": "{server}",
    "api_url": "https://filestore.me/",
    "download_url_base": "https://filestore.me/",
    "server_url": "https://filestore.me/?op=upload_form",
    "size_limit": 100,
    "size_unit": "MB"
},

"""

class FileStore:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "None :("
        try:
            ua = random.choice(user_agents)
            url = sites_data_dict[site]["api_url"]
            download_url_base = sites_data_dict[site]["download_url_base"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            api_username = api_keys.get("username", False)
            api_password = api_keys.get("password", False)

            if api_username in (False, "") or api_password in (False, ""):
                raise Exception("Missing API Credentials?")

            # Execute Login for session id
            data = {
                "op": "login",
                "login": api_username,
                "password": api_password
            }
                
            raw_req = requests.post(url=url, data=data, headers=headers, proxies=proxies)

            if raw_req.status_code == 200:
                session_id = raw_req.cookies.get("xfss")

            if calc_size == "OK":
                data = {
                    "sess_id": file_id,
                    "keepalive": 1
                }
                form_data = {
                    'file_0': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, data=data, files=form_data, headers=headers, proxies=proxies, stream=True)

                if raw_req.status_code == 200:
                    return {"status": "ok", "file_name": file_name, "file_url": download_url_base + file_id}
                else:
                    raise Exception(f"Status code: {raw_req.status_code}")
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

