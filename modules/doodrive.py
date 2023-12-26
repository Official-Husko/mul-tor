import requests
import os
import random

from .site_data import Site_Data_CLSS, sites_data_dict
from .pretty_print import *
from main import DEBUG

site = "DooDrive"

class DooDrive:
     def Uploader(file, proxy_list, user_agents, api_keys):
        """
        Uploads a file to a specified site using random user agents and proxies.

        Args:
            file (str): The path to the file to be uploaded.
            proxy_list (list): A list of proxy URLs.
            user_agents (list): A list of user agent strings.

        Returns:
            dict: A dictionary containing the status, file name, file URL, and site.

        Raises:
            Exception: If an error occurs during the upload process.
        """
        try:
            # Select a random user agent
            ua = random.choice(user_agents)
            raw_req = None
            size_limit = f'{sites_data_dict[site]["size_limit"]} {sites_data_dict[site]["size_unit"]}'

            # Get the file size and name
            file_size = os.stat(file).st_size
            file_name = os.path.basename(file)

            # Get the upload URL and size limit from the site data dictionary
            initialize_url = sites_data_dict[site]["initialize_url"]

            # Truncate the file name if it is too long
            file_name = (file_name[:240] + '..') if len(file_name) > 240 else file_name

            # Calculate the size unit for the site
            calc_size = Site_Data_CLSS.size_unit_calc(site, file_size)

            # Set the user agent header
            headers = {
                "User-Agent": ua
            }

            # Select a random proxy, if available
            proxies = random.choice(proxy_list) if proxy_list else None

            api_key = api_keys.get("apiKey", False)
            api_token = api_keys.get("apiToken", False)

            if calc_size == "OK":
                if api_key == False and api_token == False:
                    raise Exception("Missing API Keys?")
                    
                # Prepare the form data for file upload
                init_data = {
                    "api_key": api_key,
                    "api_token": api_token,
                    "file_name": file_name,
                    "file_size": file_size
                }
                # Send the upload request with the form data, headers, and proxies
                init_req = requests.post(url=initialize_url, data=init_data, headers=headers, proxies=proxies)

                # Parse the response JSON and get the download URL
                init_resp = init_req.json()
                status = init_resp.get("status", "")
                upload_token = init_resp.get("data", {}).get("token", "")
                upload_url = init_resp.get("data", {}).get("chunk_url", "")
                finalize_url = init_resp.get("data", {}).get("complete_url", "")
                chunk_size = init_resp.get("data", {}).get("max_chunk_size", "")

                if status != "success":
                    raise Exception(f"Init status: {status}")

                total_chunks = (file_size + chunk_size - 1) // chunk_size
                chunk_index = 0

                with open(file, 'rb') as file_data:
                    while True:
                        chunk_data = file_data.read(chunk_size)
                        if not chunk_data:
                            break  # Exit the loop if we've reached the end of the file
                        
                        upload_data = {
                            "api_key": api_key,
                            "api_token": api_token,
                            "token": upload_token,
                            "chunk_id": chunk_index,
                        }

                        # Prepare the json data to add extra data to the upload
                        form_data = {
                                    'file': (os.path.basename(file), chunk_data, 'application/octet-stream')
                                }

                        # Send the upload request with the form data, headers, and proxies
                        raw_req = requests.post(url=upload_url, data=upload_data, files=form_data, headers=headers, proxies=proxies, timeout=50)

                        req_resp = raw_req.json()
                        status = req_resp.get("status", "")

                        chunk_index += 1

                        if status != "success":
                            raise Exception(f"Chunk upload failed: {status}")

                upload_data = {
                    "api_key": api_key,
                    "api_token": api_token,
                    "token": upload_token,
                    "chunks": total_chunks,
                }

                raw_req = requests.post(url=finalize_url, data=upload_data, headers=headers, proxies=proxies, timeout=50)

                req_resp = raw_req.json()
                download_url = req_resp.get("data", {}).get("url", "")

                # Return successful message with the status, file name, file URL, and site
                return {"status": "ok", "file_name": file_name, "file_url": download_url}
            else:
                # Return size error message
                return {"status": "size_error", "file_name": file_name, "exception": "SIZE_ERROR", "size_limit": f"{str(size_limit)}"}
        except Exception as e:
            # Return error message
            return {"status": "error", "file_name": file_name, "exception": str(e), "extra": raw_req}

