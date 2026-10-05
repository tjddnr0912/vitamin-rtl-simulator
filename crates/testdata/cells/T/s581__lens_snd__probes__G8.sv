module top;
  localparam logic [3:0] V = {(4'b1100 ==? {2'b1?, 2'b00}){1'b1}};
  initial begin $display("G8 V=%b", V); end
endmodule
