import requests
import os
import random
import string

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "BowFile"

class BowFile:
    
    def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "None :("
        try:
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            auth_url = sites_data_dict[site]["authorize_url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            api_key1 = api_keys.get("apiKey1", False)
            api_key2 = api_keys.get("apiKey2", False)

            if calc_size == "OK":
                if api_key1 == False and api_key2 == False:
                    raise Exception("Missing API Keys?")

                auth_data = {
                    "key1": api_key1,
                    "key2": api_key2
                }

                raw_auth = requests.post(url=auth_url, data=auth_data, headers=headers, proxies=proxies)

                auth_resp = raw_auth.json()
                access_token = auth_resp.get("data", {}).get("access_token", "")
                account_id = auth_resp.get("data", {}).get("account_id", "")

                status = auth_resp.get("_status", "")

                if status != "success":
                    raise Exception("Auth Failed. Invalid API Keys?")



                form_data = {
                    'access_token': access_token,
                    'account_id': account_id
                }

                file_data = {
                    'upload_file': (os.path.basename(file), open(str(file), 'rb'), 'application/octet-stream')
                }
                
                raw_req = requests.post(url=upload_url, data=form_data, files=file_data, headers=headers, proxies=proxies, stream=True)

                response = raw_req.json()
                download_url = response.get("data", [])[0].get("url", "")

                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

