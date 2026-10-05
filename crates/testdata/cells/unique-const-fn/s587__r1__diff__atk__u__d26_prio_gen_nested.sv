module top;
  function automatic int pg(input int a);
    pg = 7;
    priority if (a == 1) pg = 10;
  endfunction
  function automatic int pg2(input int a);
    return pg(a) + 1;
  endfunction
  if (pg2(2) == 8) begin : yes
    initial #1 $display("yes");
  end else begin : no
    initial #1 $display("no");
  end
  initial #5 $finish;
endmodule
