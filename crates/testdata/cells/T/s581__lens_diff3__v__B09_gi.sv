module top;
  typedef logic [39:0] t40;
  localparam t40 PT = 40'h10_0000_000C;
  if (PT ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
