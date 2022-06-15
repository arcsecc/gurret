#!/bin/python3

import numpy as np
import matplotlib.pyplot as plt
import matplotlib.ticker as ticker
import os
import sys

FONTSIZE=80
WIDTH=10.0

def get_category_by_index(l: list[int], index: int) -> list[str]:
    cat = []
    temp = l.copy()
    while len(temp) != 0:
        first = temp[0]
        c = first.split("_")[index]
        cat.append(c)

        temp = list(filter(lambda x: c not in x, temp))
    return cat



def plot_thing(result):

    data = result
    data.sort(key=lambda x: int(x))
    

    y = []
    yerr = []

    for d in data:
        arr = [float(x[:-1]) for x in open(f"{folder}/{d}").readlines()]

        y.append(np.mean(arr))
        yerr.append( np.std(arr) / np.sqrt(len(arr)))



    avg = 0
    for i in range(1, 100):
        avg += y[i] - y[i - 1]

    print(avg /  100)


    x = [i for i in range(len(data))]


    # plt.plot(x, y)
    # plt.bar(x, y)
    # plt.errorbar(x, y,  yerr=yerr, linewidth=WIDTH, fmt="o", color="r")
    plt.errorbar(x, y,  yerr=yerr, linewidth=WIDTH)



    lim = (-5, 100)
    plt.xlim((lim))
    ax.xaxis.set_major_locator(ticker.MaxNLocator(integer=True))
    ax.tick_params(labelsize=FONTSIZE)
    ax.legend(fontsize=FONTSIZE)
    plt.legend(fontsize=FONTSIZE)

    #plt.tight_layout()
    #plt.title("test", fontsize=FONTSIZE)
    plt.show()




#plot_thing("read", categories, result)
#plot_thing("write", categories, result)


if __name__ == "__main__":
    args = sys.argv
    if len(args) != 5:
        print("Usage: ./barplot.py {title} {x label} {y label} {folder}")

    folder = args[4]

    fig, ax = plt.subplots()
    plt.xlabel(args[2], fontsize=FONTSIZE)
    plt.ylabel(args[3], fontsize=FONTSIZE)

    res = os.listdir(f"{os.getcwd()}/{folder}")

    plot_thing(res)
