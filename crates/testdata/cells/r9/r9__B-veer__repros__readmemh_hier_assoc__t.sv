module memm;
  bit [7:0] mem [int];                 // associative byte memory in a child
endmodule
module top;
  memm lmem ();
  initial begin
    $readmemh("p.hex", lmem.mem);      // $readmemh into a hierarchical associative array
    $display("b0=%h b1=%h", lmem.mem[32'h10], lmem.mem[32'h11]);
    $finish;
  end
endmodule
