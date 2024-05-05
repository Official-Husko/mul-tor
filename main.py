from time import sleep

if __name__ == '__main__':
    try:
        startup = Main.startup()
        while True:
            Main.selection(config=startup[0], available=startup[1], user_agents_list=startup[3], proxy_list=startup[2])
            print("")
            print("")
    except KeyboardInterrupt:
        print("User Cancelled")
        sleep(3)
        exit(0)
