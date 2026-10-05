module top;

  if (4'd5 inside {4'b1?00, 65'd5}) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
