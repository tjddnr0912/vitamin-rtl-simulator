module top;
  function automatic int fi2(input int n);
    fi2 = 0;
    for (int i = 0; i < n; i++) begin
      if (i < 2) fi2 += 1;
      else if (i == 7) fi2 += 100;
    end
  endfunction
  localparam int P = fi2(5);
  int n = 0;
  initial begin repeat (fi2(5)) n++; #1 $display("P=%0d n=%0d", P, n); $finish; end
endmodule
