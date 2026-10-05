module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  localparam int N2 = 2;
  if ({N2{P40}} ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
