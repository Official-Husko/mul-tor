import requests
import random
import os

from .site_data import sites_data_dict
from .pretty_print import *

site = "GoFile"

class GoFile:
    
    def Uploader(file, proxy_list, user_agents):
        try:
            ua = random.choice(user_agents)
            server_url = sites_data_dict[site]["server_url"]
            
            if proxy_list == []:
                server_res = requests.get(url=server_url, headers={"User-Agent": ua}).json()
            else:
                server_res = requests.get(url=server_url, headers={"User-Agent": ua}, proxies=random.choice(proxy_list)).json()
            server = server_res["data"]["server"]
            
            upload_url = sites_data_dict[site]["url"].format(server=server)
            
            # Get file name
            file_name = file.rsplit("\\")
            file_name = file_name[-1]
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            files_data = {'file': (os.path.basename(file), open(str(file), 'rb'), 'multipart/form-data')}
            
            if proxy_list == []:
                req = requests.post(url=upload_url, files=files_data, headers={"User-Agent": ua}).json()
            else:
                req = requests.post(url=upload_url, files=files_data, headers={"User-Agent": ua}, proxies=random.choice(proxy_list)).json()
            
            if req["status"] != "ok":
                raise Exception(req["status"])
            file_url = req["data"]["downloadPage"]
         
            return {"status": "ok", "file_name": file_name, "file_url": file_url, "site": site}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "site": site, "exception": str(e), "extra": req}