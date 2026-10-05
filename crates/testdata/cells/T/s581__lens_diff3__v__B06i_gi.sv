package pa;
  localparam logic [39:0] PW = 40'h10_0000_000C;
  localparam logic [39:0] PF = 40'h0C;
  localparam logic signed [63:0] PS = -64'sd4;
endpackage

module top;
  import pa::*;
  if (pa::PW inside {4'b1?00}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
