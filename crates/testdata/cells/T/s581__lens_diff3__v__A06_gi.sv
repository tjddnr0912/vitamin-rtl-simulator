module top;
  localparam logic [39:0] P40 = 40'h10_0000_000C;
  if ($bits(P40) ==? 8'b0010_1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
