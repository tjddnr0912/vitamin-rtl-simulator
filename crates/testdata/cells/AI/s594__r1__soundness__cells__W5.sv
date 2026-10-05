package p;
  localparam int N = 9;
  localparam logic [7:0] L = '0 | (((8'd200 + 8'd100) >> 1) + (|{N{8'hFF}}));
endpackage
module t;
  initial $display("L=%0d", p::L);
  initial #100 $finish;
endmodule
