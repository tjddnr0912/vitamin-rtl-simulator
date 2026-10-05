module top;
  localparam shortint unsigned PSU = 16'hFFFC;
  if (PSU ==? 4'sb1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
