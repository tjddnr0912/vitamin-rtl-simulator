module top;
  function automatic int finc(input int a); return a + 5; endfunction
  if (finc(7) ==? 4'b1?00) begin : g
    initial $display("GI=then");
  end else begin : g
    initial $display("GI=else");
  end
  initial #100 $finish;
endmodule
