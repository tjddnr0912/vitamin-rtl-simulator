module top;
  localparam logic [3:0] V = {(4'bx100 ==? 4'b1?00){1'b1}};
  initial begin $display("G1 V=%b", V); end
endmodule
