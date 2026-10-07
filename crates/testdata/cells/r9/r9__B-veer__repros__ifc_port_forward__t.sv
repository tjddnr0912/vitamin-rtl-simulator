interface bus_if;
  logic [7:0] d;
  logic [7:0] q;
  modport src (output d, input q);
endinterface
module leaf (bus_if.src b);
  assign b.d = b.q + 8'd1;
endmodule
module mid (bus_if.src bm);
  leaf u (.b(bm));              // an interface PORT forwarded to a child (IEEE 1800-2017 25.3)
endmodule
module top;
  bus_if bi ();
  mid m (.bm(bi));
  initial begin
    bi.q = 8'd41;
    #1 $display("d=%0d", bi.d);
    $finish;
  end
endmodule
