import os
import random
import urllib3
from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from .logger import Logger
import json

class Pixeldrain:
    
    @staticmethod
    def Uploader(file, proxy_list, user_agents, api_keys):
        try:
            site = "Pixeldrain"  # Moved the 'site' definition inside the method

            ua = random.choice(user_agents)
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            base_url = sites_data_dict[site]["download_url_base"]
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name
            
            upload_url = sites_data_dict[site]["url"].format(file_name=file_name)
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            api_key = api_keys.get("apiKey", False)

            if api_key in (False, ""):
                raise Exception("Missing API Credentials?")

            headers = {
                "User-Agent": ua, 
                "Content-Type": "application/octet-stream",
                "Cookie": f"pd_auth_key={api_key}"
            }
            
            proxy = random.choice(proxy_list) if proxy_list else None

            if calc_size == "OK":
                http = urllib3.PoolManager()
                with open(file, "rb") as file_upload:
                    req = http.urlopen('PUT', upload_url, body=file_upload, headers=headers, timeout=300)
                    if req.status != 201:
                        raise Exception(f"HTTP Error {req.status}: {req.reason}")
                    response_json = json.loads(req.data.decode('utf-8'))
                return {"status": "ok", "file_name": file_name, "file_url": base_url + response_json['id']}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": size_limit}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e)}
