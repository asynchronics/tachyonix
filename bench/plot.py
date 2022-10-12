#!/usr/bin/env python3
import sys
import numpy
import matplotlib.pyplot as plt

# USAGE:
#     plot.py DATAFILE [XLABEL [TITLE]]
#
# ARGS:
#    note: argument containing spaces must be enclosed in quotes.
#
#    DATAFILE           Space-separated data file; the first column is the
#                       parameter; the next columns are, in this order:
#                       - `async-channel` throughput
#                       - `flume` throughput
#                       - `postage` throughput
#                       - `tachyonix` throughput
#                       - `tokio-mpsc` throughput
#    XLABEL             Label of the x axis
#    YLABEL             Label of the y axis
#    TITLE              Title of the plot
#

def plot(data, x_label, title):
    WIDTH = 0.5  # total width of a group of bars
    MULTIPLIER = 1e-6 # convert y units from msg/s to msg/us

    parameter_labels = [int(param) for param in data[:,0]]
    channel_labels = ['async-channel::bounded', 'flume::bounded', 'postage::mpsc', 'tachyonix', 'tokio::mpsc']

    data = numpy.transpose(data[:, 1:])
    x = numpy.arange(len(parameter_labels))
    n_channels = len(data)

    ax = plt.subplots()[1]
    
    for i, col in enumerate(data):
        delta = -WIDTH/2.0 + i*WIDTH/(n_channels-1)
        ax.bar(x + delta, col*MULTIPLIER, WIDTH/n_channels, label=channel_labels[i])

    if title is not None:
        ax.set_title(title)
    if x_label is not None:
        ax.set_xlabel(x_label)
    ax.set_ylabel('msg/µs')
    ax.set_xticks(x)
    ax.set_xticklabels(parameter_labels)
    ax.legend(loc='upper left')

    plt.savefig('bench.png', format='png', dpi=150, bbox_inches='tight')
    plt.show()



if __name__ == '__main__':
    x_label = None
    title = None

    if len(sys.argv) >= 3:
        x_label = sys.argv[2]
    if len(sys.argv) >= 4:
        title = sys.argv[3]
    
    with open(sys.argv[1]) as f:
        data = numpy.loadtxt(f)
        plot(data, x_label, title)

