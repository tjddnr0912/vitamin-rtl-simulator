`timescale 1ns/1ps
module tbw; pcie_tlp_demux_bar #(.PORTS(4), .BAR_BASE(0), .BAR_STRIDE(0), .FIFO_DEPTH(64)) u(); initial begin #1 $display("IDS=%h", u.BAR_IDS_INT); $finish; end endmodule
