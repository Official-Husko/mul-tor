import requests
import os
import random
import string
import re
import uuid

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "Fastupload"

class Fastupload:
    
     def Uploader(file, proxy_list, user_agents, api_keys):
        raw_req = "None :("
        try:
            file_blacklist = [".exe", ".php", ".lnk", ".html", ".htm", ".phtml", ".shtm", ".js", ".bat", ".dll"]
            ua = random.choice(user_agents)
            upload_url = sites_data_dict[site]["url"]
            normal_url = sites_data_dict[site]["api_url"]
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'
            download_url_base = sites_data_dict[site]["download_url_base"]
            
            
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name # Changed from 255 to 240 as an additional safety net.
            
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            headers = {"User-Agent": ua}
            proxies = random.choice(proxy_list) if proxy_list else None

            raw_req = requests.get(url=normal_url, headers=headers, proxies=proxies, timeout=300)

            csrf_token_match = re.search(r'<meta name="csrf-token" content="(.*?)" />', raw_req.text)

            # Check if the match was found
            if csrf_token_match:
                csrf_token = csrf_token_match.group(1)
            else:
                csrf_token = False
            
            x_csrf_token = raw_req.cookies.get("XSRF-TOKEN", False)
            session = raw_req.cookies.get("filebob_user_session", False)

            if csrf_token in (False, "") or x_csrf_token in (False, "") or session in (False, ""):
                raise Exception("Failed to get CSRF Tokens. Report this!")

            _, file_extension = os.path.splitext(file_name)

            if calc_size == "OK":
                if file_extension in file_blacklist:
                    raise Exception(f"File is blacklisted! {file_blacklist}")

                headers = {
                    "User-Agent": ua,
                    "Accept": "application/json",
                    "X-CSRF-TOKEN": csrf_token,
                    "Cookie": f"XSRF-TOKEN={x_csrf_token}; filebob_user_session={session}"
                }

                chunk_size = 94371840 # 90 MB
                total_chunks = (file_size + chunk_size - 1) // chunk_size
                chunk_index = 0
                dzchunkbyteoffset = 0

                file_uuid = str(uuid.uuid4())

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        headers = {
                            "User-Agent": ua,
                            "Accept": "application/json",
                            "X-CSRF-TOKEN": csrf_token,
                            "Cookie": f"XSRF-TOKEN={x_csrf_token}; filebob_user_session={session}"
                        }

                        upload_data = {
                            "dzuuid": file_uuid,
                            "dzchunkindex": chunk_index,
                            "dztotalfilesize": file_size,
                            "dzchunksize": chunk_size,
                            "dztotalchunkcount": total_chunks,
                            "dzchunkbyteoffset": dzchunkbyteoffset,
                            "size": file_size,
                            "upload_auto_delete": 0
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'file': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=300, stream=True)

                        x_csrf_token = raw_req.cookies.get("XSRF-TOKEN", False)
                        session = raw_req.cookies.get("filebob_user_session", False)

                        chunk_index += 1
                        dzchunkbyteoffset += chunk_size

                if raw_req.status_code != 200:
                    raise Exception(f"Failed to upload file. Status Code {raw_req.status_code}. Report this!")

                req_json = raw_req.json()

                download_url = req_json.get("download_link", False)

                if download_url in (False, ""):
                    raise Exception("Failed to get download link. Report this!")

                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
                
        except Exception as e:
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

