module top #(parameter type T = logic [39:0], parameter T P = 40'h10_0000_000C);

  if (P ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
